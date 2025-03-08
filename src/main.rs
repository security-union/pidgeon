#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use actix_files::Files;
    use actix_web::*;
    use leptos::*;
    use leptos_actix::{generate_route_list, LeptosRoutes};
    use log::info;
    use pidgeoneer::app::*;

    let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
    let addr = conf.leptos_options.site_addr;
    let routes = generate_route_list(App);
    info!("starting HTTP server at http://{}", &addr);

    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;

        App::new()
            .service(Files::new("/pkg", format!("{site_root}/pkg")))
            .service(Files::new("/assets", site_root))
            .service(favicon)
            .leptos_routes(
                leptos_options.to_owned(),
                routes.to_owned(),
                App,
            )
            .app_data(web::Data::new(leptos_options.to_owned()))
    })
    .bind(&addr)?
    .run()
    .await?;

    Ok(())
}

#[cfg(feature = "ssr")]
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

#[cfg(feature = "hydrate")]
pub fn main() {
    use leptos::*;
    use wasm_bindgen::prelude::wasm_bindgen;
    use pidgeoneer::app::*;

    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug)
        .expect("error initializing log");

    logging::log!("csr mode - hydrating");

    mount_to_body(move || view! { <App/> });
} 