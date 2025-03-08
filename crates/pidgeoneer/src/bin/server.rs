use actix_files::Files;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use actix::{Actor, Addr, ActorContext, Handler, Message, StreamHandler};
use futures_util::stream::StreamExt;
use leptos::*;
use leptos_actix::{generate_route_list, LeptosRoutes};
use pidgeoneer::app::App as LeptosApp;
use pidgeoneer::app::PidControllerData;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use log::*;
use iggy::messages::poll_messages::PollingStrategy;
use iggy::clients::client::IggyClient;
use iggy::client::Client;
use iggy::client::UserClient;
use iggy::identifier::Identifier;
use iggy::consumer::Consumer;
use iggy::consumer::ConsumerKind;
use iggy::client::MessageClient;
use std::str::FromStr;

// Shared state for WebSocket connections
struct AppState {
    clients: Mutex<HashMap<usize, Addr<WebSocketSession>>>,
    client_counter: Mutex<usize>,
}

// Message for broadcasting data to all connected clients
#[derive(Message, Clone)]
#[rtype(result = "()")]
struct BroadcastPidData(PidControllerData);

// WebSocket session actor
#[derive(Clone)]
struct WebSocketSession {
    id: usize,
    app_state: Arc<AppState>,
}

impl Actor for WebSocketSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("WebSocket connection established: {}", self.id);
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> actix::Running {
        info!("WebSocket connection closed: {}", self.id);
        
        // Remove self from app state
        if let Ok(mut clients) = self.app_state.clients.lock() {
            clients.remove(&self.id);
        }
        
        actix::Running::Stop
    }
}

// Handler for WebSocket messages
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                debug!("Received text message: {}", text);
                // Echo back the message (for testing)
                ctx.text(text);
            },
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            },
            _ => (),
        }
    }
}

// Handle broadcast messages
impl Handler<BroadcastPidData> for WebSocketSession {
    type Result = ();

    fn handle(&mut self, msg: BroadcastPidData, ctx: &mut Self::Context) -> Self::Result {
        if let Ok(json) = serde_json::to_string(&msg.0) {
            ctx.text(json);
        }
    }
}

// WebSocket handler
async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    app_state: web::Data<Arc<AppState>>,
) -> Result<HttpResponse, Error> {
    // Get a new client ID
    let id = {
        let mut counter = app_state.client_counter.lock().unwrap();
        *counter += 1;
        *counter
    };
    
    // Create a new WebSocket session
    let session = WebSocketSession {
        id,
        app_state: app_state.get_ref().clone(),
    };
    
    // Handle the WebSocket connection
    let (addr, resp) = ws::start_with_addr(session, &req, stream)?;
    
    // Store the client
    if let Ok(mut clients) = app_state.clients.lock() {
        clients.insert(id, addr);
    }
    
    Ok(resp)
}

// Start Iggy consumer in a separate thread
fn start_iggy_consumer(app_state: Arc<AppState>) {
    thread::spawn(move || {
        info!("Starting Iggy consumer thread");
        
        // Create a runtime for async operations
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                error!("Failed to create tokio runtime: {}", e);
                return;
            }
        };
        
        // Setup Iggy consumer
        runtime.block_on(async {
            // Connection parameters
            let connection_string = "iggy://iggy:iggy@localhost:8090";
            
            // Create Iggy client
            info!("Connecting to Iggy server at {}", connection_string);
            let client = match iggy::clients::client::IggyClient::from_connection_string(connection_string) {
                Ok(client) => {
                    match client.connect().await {
                        Ok(_) => {
                            info!("✅ Connected to Iggy server");
                            
                            // Login with default credentials
                            if let Err(e) = client.login_user("iggy", "iggy").await {
                                error!("Failed to login to Iggy: {}", e);
                                return;
                            }
                            
                            client
                        },
                        Err(e) => {
                            error!("Failed to connect to Iggy server: {}", e);
                            return;
                        }
                    }
                },
                Err(e) => {
                    error!("❌ Failed to create Iggy client: {}", e);
                    return;
                }
            };
            
            // Create a consumer
            let stream_name = Identifier::from_str("pidgeon_debug").unwrap();
            let topic_name = Identifier::from_str("controller_data").unwrap();
            
            let consumer = Consumer {
                kind: ConsumerKind::from_code(1).unwrap(),
                id: Identifier::numeric(1).unwrap(),
            };
            
            // Start consuming messages
            info!("Starting message consumption loop");
            loop {
                // Poll for messages
                match client.poll_messages(
                    &stream_name,
                    &topic_name,
                    None,
                    &consumer,
                    &PollingStrategy::next(),
                    1,
                    true,
                ).await {
                    Ok(messages) => {
                        // for message in messages {
                        //     // Try to deserialize the message
                        //     if let Ok(payload_str) = std::str::from_utf8(&message.payload) {
                        //         match serde_json::from_str::<PidControllerData>(payload_str) {
                        //             Ok(pid_data) => {
                        //                 info!("📥 Received PID data from controller: {}", pid_data.controller_id);
                                        
                        //                 // Broadcast to all connected clients
                        //                 if let Ok(clients) = app_state.clients.lock() {
                        //                     for (_, client) in clients.iter() {
                        //                         client.do_send(BroadcastPidData(pid_data.clone()));
                        //                     }
                        //                 }
                        //             },
                        //             Err(e) => {
                        //                 error!("Failed to parse message as PidControllerData: {}", e);
                        //                 debug!("Raw message: {}", payload_str);
                        //             }
                        //         }
                        //     }
                        // }
                    },
                    Err(e) => {
                        error!("Error polling for messages: {}", e);
                        // Add a short delay to prevent CPU spinning on repeated errors
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
                
                // Small delay between polling attempts
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
    });
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Set up logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Load configuration
    let conf = get_configuration(Some("Leptos.toml")).await.unwrap();
    let addr = conf.leptos_options.site_addr;

    // Generate routes
    let routes = generate_route_list(LeptosApp);

    // Create shared application state
    let app_state = Arc::new(AppState {
        clients: Mutex::new(HashMap::new()),
        client_counter: Mutex::new(0),
    });
    
    // Start the Iggy consumer in a background thread
    start_iggy_consumer(app_state.clone());

    info!("Starting server at http://{}", addr);

    // Start HTTP server
    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;
        
        // Clone application state for this worker
        let app_state = app_state.clone();

        App::new()
            // Add shared state
            .app_data(web::Data::new(app_state.clone()))
            // WebSocket route
            .route("/ws", web::get().to(ws_route))
            // Serve static files
            .service(Files::new("/pkg", format!("{site_root}/pkg")))
            .service(Files::new("/assets", site_root))
            // Serve favicon
            .service(favicon)
            // Set up Leptos routes
            .leptos_routes(
                leptos_options.to_owned(),
                routes.to_owned(),
                LeptosApp
            )
            .app_data(web::Data::new(leptos_options.to_owned()))
    })
    .bind(&addr)?
    .run()
    .await
}

#[actix_web::get("favicon.ico")]
async fn favicon(
    leptos_options: web::Data<LeptosOptions>,
) -> actix_web::Result<actix_files::NamedFile> {
    let leptos_options = leptos_options.into_inner();
    let site_root = &leptos_options.site_root;
    Ok(actix_files::NamedFile::open(format!(
        "{site_root}/favicon.ico"
    ))?)
}
