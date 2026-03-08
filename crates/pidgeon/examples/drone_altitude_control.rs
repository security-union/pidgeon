use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use pidgeon::{ControllerConfig, ThreadSafePidController};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{
    io, thread,
    time::{Duration, Instant},
};

// Simulation constants
const CONTROL_RATE_HZ: f64 = 20.0;
const DT: f64 = 1.0 / CONTROL_RATE_HZ;
const INITIAL_SETPOINT: f64 = 10.0;
const SETPOINT_STEP: f64 = 2.0;
const MAX_SETPOINT: f64 = 25.0;
const MIN_SETPOINT: f64 = 0.0;
const WIND_GUST_STRENGTH: f64 = 4.0;
const MAX_HISTORY_SECONDS: f64 = 60.0;
const MAX_HISTORY_POINTS: usize = (MAX_HISTORY_SECONDS * CONTROL_RATE_HZ) as usize;

// PID gain adjustment step
const GAIN_STEP: f64 = 1.0;

// Missions
struct Mission {
    target: f64,
    description: &'static str,
    trigger_time: f64,
}

const MISSIONS: &[Mission] = &[
    Mission {
        target: 10.0,
        description: "Take off! Reach 10m cruise altitude",
        trigger_time: 0.0,
    },
    Mission {
        target: 18.0,
        description: "Climb to 18m to clear the treeline",
        trigger_time: 15.0,
    },
    Mission {
        target: 8.0,
        description: "Descend to 8m for aerial photography",
        trigger_time: 35.0,
    },
    Mission {
        target: 22.0,
        description: "Emergency climb to 22m! Obstacle ahead",
        trigger_time: 55.0,
    },
    Mission {
        target: 5.0,
        description: "Begin landing approach — descend to 5m",
        trigger_time: 80.0,
    },
    Mission {
        target: 0.0,
        description: "Touch down! Land the drone safely",
        trigger_time: 100.0,
    },
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up terminal
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Show intro screen
    loop {
        terminal.draw(draw_intro)?;
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        disable_raw_mode()?;
                        io::stdout().execute(LeaveAlternateScreen)?;
                        return Ok(());
                    }
                    _ => break,
                }
            }
        }
    }

    // PID gains (mutable — user can tweak at runtime)
    let mut kp = 10.0_f64;
    let mut ki = 5.0_f64;
    let mut kd = 8.0_f64;

    // Set up PID controller
    let config = ControllerConfig::builder()
        .with_kp(kp)
        .with_ki(ki)
        .with_kd(kd)
        .with_output_limits(0.0, 100.0)
        .with_setpoint(INITIAL_SETPOINT)
        .with_deadband(0.0)
        .with_anti_windup(true)
        .build()
        .expect("Invalid PID config");

    let controller = ThreadSafePidController::new(config);

    // Drone physics
    let gravity = 9.81;
    let drone_mass = 1.2;
    let max_thrust = 30.0;
    let drag_coefficient = 0.3;
    let motor_response_delay = 0.1;

    // State
    let mut altitude = 0.0;
    let mut velocity: f64 = 0.0;
    let mut commanded_thrust = 0.0;
    let mut setpoint = INITIAL_SETPOINT;
    let mut pending_gust: Option<f64> = None;
    let mut auto_missions = true;

    // PID term tracking (computed locally to mirror the controller)
    let mut integral_accum = 0.0_f64;
    let mut prev_measurement = 0.0_f64;
    let mut first_run = true;

    // Mission tracking
    let mut current_mission_idx: usize = 0;
    let mut mission_text = MISSIONS[0].description.to_string();

    // Data history
    let mut altitude_data: Vec<(f64, f64)> = Vec::new();
    let mut setpoint_data: Vec<(f64, f64)> = Vec::new();
    let mut velocity_data: Vec<(f64, f64)> = Vec::new();
    let mut thrust_data: Vec<(f64, f64)> = Vec::new();
    let mut error_data: Vec<(f64, f64)> = Vec::new();
    let mut p_term_data: Vec<(f64, f64)> = Vec::new();
    let mut i_term_data: Vec<(f64, f64)> = Vec::new();
    let mut d_term_data: Vec<(f64, f64)> = Vec::new();
    let mut event_log: Vec<(f64, String)> = Vec::new();

    let mut current_event = "Engines starting...".to_string();
    let mut time_step: usize = 0;
    let mut show_help = false;
    let mut paused_duration = Duration::ZERO;
    let mut pause_start: Option<Instant> = None;
    let start = Instant::now();

    loop {
        // Handle input
        if event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if show_help {
                    if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                        break;
                    }
                    show_help = false;
                    if let Some(ps) = pause_start.take() {
                        paused_duration += ps.elapsed();
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Up => {
                            auto_missions = false;
                            setpoint = (setpoint + SETPOINT_STEP).min(MAX_SETPOINT);
                            controller
                                .set_setpoint(setpoint)
                                .expect("Failed to set setpoint");
                            let msg = format!("Manual override: target {setpoint:.0}m");
                            current_event = msg.clone();
                            event_log.push((time_step as f64 * DT, msg));
                            mission_text = format!("MANUAL: Hold at {setpoint:.0}m");
                        }
                        KeyCode::Down => {
                            auto_missions = false;
                            setpoint = (setpoint - SETPOINT_STEP).max(MIN_SETPOINT);
                            controller
                                .set_setpoint(setpoint)
                                .expect("Failed to set setpoint");
                            let msg = format!("Manual override: target {setpoint:.0}m");
                            current_event = msg.clone();
                            event_log.push((time_step as f64 * DT, msg));
                            mission_text = format!("MANUAL: Hold at {setpoint:.0}m");
                        }
                        KeyCode::Char('w') => {
                            pending_gust = Some(WIND_GUST_STRENGTH);
                            let msg = format!("WIND GUST (up, +{WIND_GUST_STRENGTH:.1} m/s)");
                            current_event = msg.clone();
                            event_log.push((time_step as f64 * DT, msg));
                        }
                        KeyCode::Char('s') => {
                            pending_gust = Some(-WIND_GUST_STRENGTH);
                            let msg = format!("WIND GUST (down, -{WIND_GUST_STRENGTH:.1} m/s)");
                            current_event = msg.clone();
                            event_log.push((time_step as f64 * DT, msg));
                        }
                        // PID gain controls
                        KeyCode::Char('1') => {
                            kp = (kp + GAIN_STEP).min(50.0);
                            controller.set_kp(kp).expect("Failed to set Kp");
                            let msg = format!("Kp = {kp:.1}");
                            current_event = msg.clone();
                            event_log.push((time_step as f64 * DT, msg));
                        }
                        KeyCode::Char('2') => {
                            kp = (kp - GAIN_STEP).max(0.0);
                            controller.set_kp(kp).expect("Failed to set Kp");
                            let msg = format!("Kp = {kp:.1}");
                            current_event = msg.clone();
                            event_log.push((time_step as f64 * DT, msg));
                        }
                        KeyCode::Char('3') => {
                            ki = (ki + GAIN_STEP).min(50.0);
                            controller.set_ki(ki).expect("Failed to set Ki");
                            let msg = format!("Ki = {ki:.1}");
                            current_event = msg.clone();
                            event_log.push((time_step as f64 * DT, msg));
                        }
                        KeyCode::Char('4') => {
                            ki = (ki - GAIN_STEP).max(0.0);
                            controller.set_ki(ki).expect("Failed to set Ki");
                            let msg = format!("Ki = {ki:.1}");
                            current_event = msg.clone();
                            event_log.push((time_step as f64 * DT, msg));
                        }
                        KeyCode::Char('5') => {
                            kd = (kd + GAIN_STEP).min(50.0);
                            controller.set_kd(kd).expect("Failed to set Kd");
                            let msg = format!("Kd = {kd:.1}");
                            current_event = msg.clone();
                            event_log.push((time_step as f64 * DT, msg));
                        }
                        KeyCode::Char('6') => {
                            kd = (kd - GAIN_STEP).max(0.0);
                            controller.set_kd(kd).expect("Failed to set Kd");
                            let msg = format!("Kd = {kd:.1}");
                            current_event = msg.clone();
                            event_log.push((time_step as f64 * DT, msg));
                        }
                        KeyCode::Char('h') => {
                            show_help = true;
                            pause_start = Some(Instant::now());
                        }
                        KeyCode::Char('m') => {
                            auto_missions = true;
                            current_event = "Auto-pilot re-engaged".into();
                            event_log
                                .push((time_step as f64 * DT, "Auto-pilot re-engaged".to_string()));
                        }
                        _ => {}
                    }
                }
            }
        }

        // Pause simulation while help is shown
        if show_help {
            terminal.draw(draw_intro)?;
            thread::sleep(Duration::from_millis(50));
            continue;
        }

        let time = time_step as f64 * DT;

        // Check for mission triggers
        if auto_missions && current_mission_idx < MISSIONS.len() {
            let mission = &MISSIONS[current_mission_idx];
            if time >= mission.trigger_time {
                setpoint = mission.target;
                controller
                    .set_setpoint(setpoint)
                    .expect("Failed to set setpoint");
                mission_text = mission.description.to_string();
                let msg = format!("MISSION: {}", mission.description);
                current_event = msg.clone();
                event_log.push((time, msg));
                current_mission_idx += 1;
            }
        }

        // Apply pending wind gust
        if let Some(gust) = pending_gust.take() {
            velocity += gust;
        }

        // PID compute
        let control_signal = controller.compute(altitude, DT).expect("Failed to compute");

        // Compute PID terms locally (mirrors the controller's internal math)
        let signed_error = setpoint - altitude;
        let p_term = kp * signed_error;
        integral_accum += ki * signed_error * DT;
        // Clamp integral to output range to approximate anti-windup
        integral_accum = integral_accum.clamp(0.0, 100.0);
        let i_term = integral_accum;
        let d_term = if first_run {
            first_run = false;
            0.0
        } else {
            // Derivative-on-measurement (negated)
            let raw_deriv = -(altitude - prev_measurement) / DT;
            kd * raw_deriv
        };
        prev_measurement = altitude;

        // Motor response delay
        commanded_thrust += (control_signal - commanded_thrust) * DT / motor_response_delay;

        // Convert to Newtons
        let thrust = commanded_thrust * max_thrust / 100.0;

        // Physics
        let weight = drone_mass * gravity;
        let drag = drag_coefficient * velocity.abs() * velocity;
        let accel = (thrust - weight - drag) / drone_mass;
        velocity += accel * DT;
        altitude += velocity * DT;
        if altitude < 0.0 {
            altitude = 0.0;
            velocity = 0.0;
        }

        let error = (setpoint - altitude).abs();

        // Record data (with rolling window)
        altitude_data.push((time, altitude));
        setpoint_data.push((time, setpoint));
        velocity_data.push((time, velocity));
        thrust_data.push((time, commanded_thrust));
        error_data.push((time, setpoint - altitude));
        p_term_data.push((time, p_term));
        i_term_data.push((time, i_term));
        d_term_data.push((time, d_term));

        if altitude_data.len() > MAX_HISTORY_POINTS {
            altitude_data.remove(0);
            setpoint_data.remove(0);
            velocity_data.remove(0);
            thrust_data.remove(0);
            error_data.remove(0);
            p_term_data.remove(0);
            i_term_data.remove(0);
            d_term_data.remove(0);
        }

        // Auto-detect flight state
        if !current_event.contains("WIND")
            && !current_event.contains("MISSION")
            && !current_event.contains("Manual")
            && !current_event.contains("Auto-pilot")
            && !current_event.starts_with("K")
        {
            current_event = if error < 0.5 {
                "On target".into()
            } else if control_signal > 50.0 {
                "Ascending (high power)".into()
            } else if control_signal > 42.0 {
                "Ascending".into()
            } else if control_signal < 38.0 {
                "Descending".into()
            } else {
                "Hovering".into()
            };
        }

        time_step += 1;

        // Render at ~10fps
        if time_step.is_multiple_of(2) {
            let time_min = if time > MAX_HISTORY_SECONDS {
                time - MAX_HISTORY_SECONDS
            } else {
                0.0
            };
            terminal.draw(|f| {
                if show_help {
                    draw_intro(f);
                } else {
                    draw_ui(
                        f,
                        time,
                        setpoint,
                        altitude,
                        velocity,
                        commanded_thrust,
                        kp,
                        ki,
                        kd,
                        &current_event,
                        &mission_text,
                        &altitude_data,
                        &setpoint_data,
                        &velocity_data,
                        &thrust_data,
                        &error_data,
                        &p_term_data,
                        &i_term_data,
                        &d_term_data,
                        &event_log,
                        time_min,
                        time,
                        auto_missions,
                    );
                }
            })?;
        }

        // Pace to real time (subtract time spent paused)
        let expected = Duration::from_secs_f64(time_step as f64 * DT);
        let elapsed = start.elapsed().saturating_sub(paused_duration);
        if expected > elapsed {
            thread::sleep(expected - elapsed);
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    // Print stats
    let stats = controller
        .get_statistics()
        .expect("Failed to get statistics");

    println!("\nDrone Altitude Control — Results");
    println!("================================");
    println!("PID gains:       Kp={kp:.1}  Ki={ki:.1}  Kd={kd:.1}");
    println!("Final setpoint:  {:.1} m", setpoint);
    println!("Final altitude:  {:.2} m", altitude);
    println!("Average error:   {:.2} m", stats.average_error);
    println!("Max overshoot:   {:.2} m", stats.max_overshoot);
    println!("Rise time:       {:.1} s", stats.rise_time);
    println!("Settling time:   {:.1} s", stats.settling_time);

    Ok(())
}

fn draw_intro(f: &mut Frame) {
    let area = f.area();

    let layout = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(26),
        Constraint::Min(0),
    ])
    .split(area);

    let center = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(62),
        Constraint::Min(0),
    ])
    .split(layout[1]);

    let content_area = center[1];

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  DRONE ALTITUDE CONTROL",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  A PID Controller Demo",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  You are the flight operator of an autonomous drone.",
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "  A PID controller keeps it stable — your job is to",
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "  give it targets, throw wind at it, and tune the",
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "  controller gains to see how they affect behavior.",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  FLIGHT:  ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Up/Down", Style::default().fg(Color::Cyan)),
            Span::raw(" target  "),
            Span::styled("W/S", Style::default().fg(Color::Red)),
            Span::raw(" wind  "),
            Span::styled("M", Style::default().fg(Color::Green)),
            Span::raw(" auto"),
        ]),
        Line::from(vec![
            Span::styled(
                "  TUNING:  ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("1/2", Style::default().fg(Color::LightRed)),
            Span::raw(" Kp +/-  "),
            Span::styled("3/4", Style::default().fg(Color::LightGreen)),
            Span::raw(" Ki +/-  "),
            Span::styled("5/6", Style::default().fg(Color::LightBlue)),
            Span::raw(" Kd +/-"),
        ]),
        Line::from(vec![
            Span::raw("           "),
            Span::styled("H", Style::default().fg(Color::White)),
            Span::raw(" help  "),
            Span::styled("Q", Style::default().fg(Color::DarkGray)),
            Span::raw(" quit"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  TRY: Set Ki=0 and Kd=0, then watch the drone",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            "  oscillate with only proportional control.",
            Style::default().fg(Color::Gray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Press H during flight to show this help again.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Press any key to fly ...",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK),
        )),
        Line::from(""),
    ];

    let intro = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" pidgeon ")
                .title_alignment(Alignment::Center),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(intro, content_area);
}

#[allow(clippy::too_many_arguments)]
fn draw_ui(
    f: &mut Frame,
    time: f64,
    setpoint: f64,
    altitude: f64,
    velocity: f64,
    thrust: f64,
    kp: f64,
    ki: f64,
    kd: f64,
    event: &str,
    mission: &str,
    altitude_data: &[(f64, f64)],
    setpoint_data: &[(f64, f64)],
    velocity_data: &[(f64, f64)],
    thrust_data: &[(f64, f64)],
    error_data: &[(f64, f64)],
    p_term_data: &[(f64, f64)],
    i_term_data: &[(f64, f64)],
    d_term_data: &[(f64, f64)],
    event_log: &[(f64, String)],
    time_min: f64,
    time_max: f64,
    auto_mode: bool,
) {
    let area = f.area();

    let main_layout = Layout::vertical([
        Constraint::Length(3), // mission
        Constraint::Length(3), // status
        Constraint::Length(3), // controls
        Constraint::Min(0),    // charts
        Constraint::Length(5), // event log
    ])
    .split(area);

    // Mission bar
    let mode_indicator = if auto_mode {
        Span::styled(" AUTO ", Style::default().fg(Color::Black).bg(Color::Green))
    } else {
        Span::styled(
            " MANUAL ",
            Style::default().fg(Color::Black).bg(Color::Magenta),
        )
    };

    let mission_line = Line::from(vec![
        Span::raw(" "),
        mode_indicator,
        Span::raw("  "),
        Span::styled(
            mission,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let mission_widget = Paragraph::new(mission_line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Mission "),
    );
    f.render_widget(mission_widget, main_layout[0]);

    // Status bar with PID gains
    let error = (setpoint - altitude).abs();
    let alt_color = if error < 0.5 {
        Color::Green
    } else if error < 2.0 {
        Color::Yellow
    } else {
        Color::Red
    };

    let event_color = if event.contains("WIND") {
        Color::Red
    } else if event.contains("On target") {
        Color::Green
    } else {
        Color::Gray
    };

    let status_line = Line::from(vec![
        Span::raw(format!(" t={:.1}s  Alt=", time)),
        Span::styled(format!("{:.1}m", altitude), Style::default().fg(alt_color)),
        Span::raw(format!(
            "/{:.0}m  Vel={:.1}  Thr={:.0}%  ",
            setpoint, velocity, thrust
        )),
        Span::styled(format!("Kp={kp:.0} "), Style::default().fg(Color::LightRed)),
        Span::styled(
            format!("Ki={ki:.0} "),
            Style::default().fg(Color::LightGreen),
        ),
        Span::styled(
            format!("Kd={kd:.0}  "),
            Style::default().fg(Color::LightBlue),
        ),
        Span::styled(event, Style::default().fg(event_color)),
    ]);

    let status = Paragraph::new(status_line).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Flight Status "),
    );
    f.render_widget(status, main_layout[1]);

    // Controls bar
    let controls = Paragraph::new(Line::from(vec![
        Span::styled(
            " Up/Down ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Alt  "),
        Span::styled(
            " W/S ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw("Wind  "),
        Span::styled(
            " 1/2 ",
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Kp  "),
        Span::styled(
            " 3/4 ",
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Ki  "),
        Span::styled(
            " 5/6 ",
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Kd  "),
        Span::styled(
            " M ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Auto  "),
        Span::styled(
            " H ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Help  "),
        Span::styled(
            " Q ",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Quit"),
    ]))
    .block(Block::default().borders(Borders::ALL).title(" Controls "));
    f.render_widget(controls, main_layout[2]);

    // 2x2 chart grid
    let rows = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_layout[3]);
    let top_cols =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(rows[0]);
    let bot_cols =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(rows[1]);

    let x_labels = vec![
        Span::raw(format!("{:.0}s", time_min)),
        Span::raw(format!("{:.0}s", (time_min + time_max) / 2.0)),
        Span::raw(format!("{:.0}s", time_max)),
    ];

    // ── Chart 1: Altitude + Setpoint (top-left) ──
    let alt_data_max = altitude_data
        .iter()
        .map(|(_, v)| *v)
        .fold(0.0_f64, f64::max);
    let sp_data_max = setpoint_data
        .iter()
        .map(|(_, v)| *v)
        .fold(0.0_f64, f64::max);
    let alt_y_max = alt_data_max.max(sp_data_max).max(5.0) + 3.0;
    let alt_y_max = (alt_y_max / 5.0).ceil() * 5.0;
    let alt_y_labels: Vec<String> = (0..=alt_y_max as i32)
        .step_by(5)
        .map(|v| format!("{v}"))
        .collect();
    render_dual_chart(
        f,
        top_cols[0],
        "Altitude (m)",
        altitude_data,
        Color::Yellow,
        "Target",
        setpoint_data,
        Color::Red,
        time_min,
        time_max,
        0.0,
        alt_y_max,
        &x_labels,
        &alt_y_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    );

    // ── Chart 2: Velocity + Thrust (top-right) ──
    // Normalize thrust to velocity scale for overlay
    let vel_abs_max = velocity_data
        .iter()
        .map(|(_, v)| v.abs())
        .fold(4.0_f64, f64::max);
    let vel_bound = (vel_abs_max + 2.0).ceil();
    render_dual_chart(
        f,
        top_cols[1],
        "Velocity (m/s)",
        velocity_data,
        Color::Cyan,
        "Thrust (%)",
        thrust_data,
        Color::Red,
        time_min,
        time_max,
        -vel_bound,
        vel_bound.max(100.0),
        &x_labels,
        &[
            &format!("{:.0}", -vel_bound),
            "0",
            "50",
            &format!("{:.0}", vel_bound.max(100.0)),
        ],
    );

    // ── Chart 3: Error (bottom-left) ──
    let err_abs_max = error_data
        .iter()
        .map(|(_, v)| v.abs())
        .fold(5.0_f64, f64::max);
    let err_bound = (err_abs_max + 2.0).ceil();
    render_chart(
        f,
        bot_cols[0],
        "Error (m)",
        error_data,
        Color::Green,
        time_min,
        time_max,
        -err_bound,
        err_bound,
        &x_labels,
        &[
            &format!("{:.0}", -err_bound),
            "0",
            &format!("{:.0}", err_bound),
        ],
    );

    // ── Chart 4: PID Terms (bottom-right) ──
    let pid_abs_max = p_term_data
        .iter()
        .chain(i_term_data.iter())
        .chain(d_term_data.iter())
        .map(|(_, v)| v.abs())
        .fold(10.0_f64, f64::max);
    let pid_bound = (pid_abs_max + 5.0).ceil();
    let pid_bound = (pid_bound / 10.0).ceil() * 10.0; // round to 10s
    render_pid_chart(
        f,
        bot_cols[1],
        p_term_data,
        i_term_data,
        d_term_data,
        time_min,
        time_max,
        -pid_bound,
        pid_bound,
        &x_labels,
        &[
            &format!("{:.0}", -pid_bound),
            "0",
            &format!("{:.0}", pid_bound),
        ],
    );

    // Event log
    let recent_events: Vec<Line> = event_log
        .iter()
        .rev()
        .take(3)
        .map(|(t, msg)| {
            let color = if msg.contains("WIND") {
                Color::Red
            } else if msg.contains("MISSION") {
                Color::Cyan
            } else if msg.contains("Auto-pilot") {
                Color::Green
            } else if msg.starts_with("K") {
                Color::Yellow
            } else {
                Color::Magenta
            };
            Line::from(vec![
                Span::styled(
                    format!(" [{:.1}s] ", t),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(msg.clone(), Style::default().fg(color)),
            ])
        })
        .collect();

    let log = Paragraph::new(recent_events)
        .block(Block::default().borders(Borders::ALL).title(" Event Log "));
    f.render_widget(log, main_layout[4]);
}

fn render_dual_chart(
    f: &mut Frame,
    area: Rect,
    name1: &str,
    data1: &[(f64, f64)],
    color1: Color,
    name2: &str,
    data2: &[(f64, f64)],
    color2: Color,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    x_labels: &[Span],
    y_labels: &[&str],
) {
    let datasets = vec![
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(color1))
            .data(data1),
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(color2))
            .data(data2),
    ];

    let y_label_spans: Vec<Span> = y_labels.iter().map(|s| Span::raw(*s)).collect();

    let title_line = Line::from(vec![
        Span::raw(" "),
        Span::styled(
            format!("\u{2588} {name1}"),
            Style::default().fg(color1).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("\u{2588} {name2}"),
            Style::default().fg(color2).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ]);

    let chart = Chart::new(datasets)
        .block(Block::default().borders(Borders::ALL).title(title_line))
        .x_axis(
            Axis::default()
                .title("Time")
                .style(Style::default().fg(Color::Gray))
                .bounds([x_min, x_max])
                .labels(x_labels.to_vec()),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::Gray))
                .bounds([y_min, y_max])
                .labels(y_label_spans),
        );

    f.render_widget(chart, area);
}

fn render_pid_chart(
    f: &mut Frame,
    area: Rect,
    p_data: &[(f64, f64)],
    i_data: &[(f64, f64)],
    d_data: &[(f64, f64)],
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    x_labels: &[Span],
    y_labels: &[&str],
) {
    let datasets = vec![
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::LightRed))
            .data(p_data),
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::LightGreen))
            .data(i_data),
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::LightBlue))
            .data(d_data),
    ];

    let y_label_spans: Vec<Span> = y_labels.iter().map(|s| Span::raw(*s)).collect();

    let title_line = Line::from(vec![
        Span::raw(" PID Terms: "),
        Span::styled(
            "\u{2588} P",
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            "\u{2588} I",
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            "\u{2588} D",
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ]);

    let chart = Chart::new(datasets)
        .block(Block::default().borders(Borders::ALL).title(title_line))
        .x_axis(
            Axis::default()
                .title("Time")
                .style(Style::default().fg(Color::Gray))
                .bounds([x_min, x_max])
                .labels(x_labels.to_vec()),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::Gray))
                .bounds([y_min, y_max])
                .labels(y_label_spans),
        );

    f.render_widget(chart, area);
}

fn render_chart(
    f: &mut Frame,
    area: Rect,
    title: &str,
    data: &[(f64, f64)],
    color: Color,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    x_labels: &[Span],
    y_labels: &[&str],
) {
    let datasets = vec![Dataset::default()
        .marker(symbols::Marker::Braille)
        .style(Style::default().fg(color))
        .data(data)];

    let y_label_spans: Vec<Span> = y_labels.iter().map(|s| Span::raw(*s)).collect();

    let title_line = Line::from(vec![
        Span::raw(" "),
        Span::styled(
            format!("\u{2588} {title}"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ]);

    let chart = Chart::new(datasets)
        .block(Block::default().borders(Borders::ALL).title(title_line))
        .x_axis(
            Axis::default()
                .title("Time")
                .style(Style::default().fg(Color::Gray))
                .bounds([x_min, x_max])
                .labels(x_labels.to_vec()),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::Gray))
                .bounds([y_min, y_max])
                .labels(y_label_spans),
        );

    f.render_widget(chart, area);
}
