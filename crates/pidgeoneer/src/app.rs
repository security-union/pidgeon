use crate::models::PidControllerData;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

#[cfg(feature = "hydrate")]
const MAX_CHART_POINTS: usize = 300;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1.0"/>
                <title>Pidgeoneer - PID Controller Dashboard</title>
                <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
                <style>
                    {r#"
                    * { box-sizing: border-box; margin: 0; padding: 0; }

                    body {
                        font-family: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                        background: #0f1117;
                        color: #e0e0e0;
                        line-height: 1.5;
                    }

                    header {
                        background: #1a1d28;
                        padding: 12px 24px;
                        display: flex;
                        justify-content: space-between;
                        align-items: center;
                        border-bottom: 1px solid #2a2d3a;
                    }

                    header h1 {
                        font-size: 1.4rem;
                        font-weight: 600;
                        color: #fff;
                    }

                    .status {
                        padding: 4px 12px;
                        border-radius: 12px;
                        font-size: 0.7rem;
                        font-weight: 600;
                        text-transform: uppercase;
                        letter-spacing: 0.05em;
                    }
                    .connected { background: #22c55e; color: #fff; }
                    .disconnected { background: #f59e0b; color: #1a1a2e; }

                    .intro {
                        padding: 20px 24px 8px;
                    }

                    .intro h2 {
                        font-size: 1.1rem;
                        font-weight: 600;
                        color: #fff;
                        margin-bottom: 8px;
                    }

                    .intro p {
                        font-size: 0.85rem;
                        color: #999;
                        max-width: 900px;
                    }

                    .intro p + p {
                        margin-top: 6px;
                    }

                    .intro a {
                        color: #3b82f6;
                        text-decoration: none;
                    }

                    .intro strong {
                        color: #ccc;
                    }

                    .metrics {
                        display: grid;
                        grid-template-columns: repeat(4, 1fr);
                        gap: 12px;
                        padding: 16px 24px;
                    }

                    .metric-card {
                        background: #1a1d28;
                        border-radius: 8px;
                        padding: 14px 16px;
                        border: 1px solid #2a2d3a;
                    }

                    .metric-label {
                        font-size: 0.7rem;
                        color: #888;
                        text-transform: uppercase;
                        letter-spacing: 0.05em;
                        display: block;
                        margin-bottom: 4px;
                    }

                    .metric-sublabel {
                        font-size: 0.65rem;
                        color: #555;
                        display: block;
                        margin-top: 2px;
                    }

                    .metric-value {
                        font-size: 1.6rem;
                        font-weight: 700;
                        color: #fff;
                    }

                    .charts {
                        padding: 0 24px 24px;
                        display: flex;
                        flex-direction: column;
                        gap: 12px;
                    }

                    .chart-panel {
                        background: #1a1d28;
                        border-radius: 8px;
                        padding: 16px;
                        border: 1px solid #2a2d3a;
                    }

                    .chart-header {
                        display: flex;
                        justify-content: space-between;
                        align-items: baseline;
                        margin-bottom: 8px;
                    }

                    .chart-header h2 {
                        font-size: 0.85rem;
                        font-weight: 600;
                        color: #ccc;
                    }

                    .chart-hint {
                        font-size: 0.7rem;
                        color: #555;
                        font-style: italic;
                    }

                    .chart-desc {
                        font-size: 0.75rem;
                        color: #666;
                        margin-bottom: 10px;
                    }

                    .chart-wrapper {
                        position: relative;
                        height: 220px;
                    }

                    .what-to-look-for {
                        background: #1a1d28;
                        border-radius: 8px;
                        padding: 16px 20px;
                        border: 1px solid #2a2d3a;
                        margin: 0 24px 24px;
                    }

                    .what-to-look-for h3 {
                        font-size: 0.8rem;
                        font-weight: 600;
                        color: #ccc;
                        text-transform: uppercase;
                        letter-spacing: 0.05em;
                        margin-bottom: 10px;
                    }

                    .what-to-look-for ul {
                        list-style: none;
                        padding: 0;
                    }

                    .what-to-look-for li {
                        font-size: 0.8rem;
                        color: #888;
                        padding: 4px 0;
                        padding-left: 16px;
                        position: relative;
                    }

                    .what-to-look-for li::before {
                        content: '';
                        position: absolute;
                        left: 0;
                        top: 10px;
                        width: 6px;
                        height: 6px;
                        border-radius: 50%;
                        background: #3b82f6;
                    }

                    .what-to-look-for li strong {
                        color: #ccc;
                    }

                    .pid-formula {
                        background: #12141c;
                        border: 1px solid #2a2d3a;
                        border-radius: 6px;
                        padding: 12px 16px;
                        margin: 12px 24px;
                        font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
                        font-size: 0.8rem;
                        color: #aaa;
                        text-align: center;
                        letter-spacing: 0.02em;
                    }

                    .pid-formula span.p { color: #3b82f6; }
                    .pid-formula span.i { color: #ef4444; }
                    .pid-formula span.d { color: #22c55e; }
                    .pid-formula span.eq { color: #666; }

                    @media (max-width: 768px) {
                        .metrics { grid-template-columns: repeat(2, 1fr); }
                    }
                    "#}
                </style>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let (pid_data, set_pid_data) = signal(Vec::<PidControllerData>::new());
    let (connected, set_connected) = signal(false);

    #[cfg(feature = "hydrate")]
    {
        use crate::iggy_client::IggyClient;

        let set_connected_clone = set_connected.clone();

        let on_open = move || {
            set_connected_clone.set(true);
        };

        let on_close = move || {
            set_connected.set(false);
        };

        let _iggy_client = IggyClient::new(set_pid_data, on_open, on_close);
    }

    #[cfg(not(feature = "hydrate"))]
    {
        let _ = set_pid_data;
        let _ = set_connected;
    }

    view! {
        <Stylesheet id="leptos" href="/pkg/pidgeoneer.css"/>
        <Title text="Pidgeoneer - PID Controller Dashboard"/>

        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=move || view! {
                        <HomePage
                            pid_data=pid_data
                            connected=connected
                        />
                    }/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage(
    pid_data: ReadSignal<Vec<PidControllerData>>,
    connected: ReadSignal<bool>,
) -> impl IntoView {
    // Set up chart update effect (client-side only)
    #[cfg(feature = "hydrate")]
    {
        setup_chart_functions();

        leptos::prelude::Effect::new(move |_| {
            let data = pid_data.get();
            if !data.is_empty() {
                update_all_charts(&data);
            }
        });
    }

    view! {
        <header>
            <h1>"Pidgeoneer"</h1>
            <div class={move || if connected.get() { "status connected" } else { "status disconnected" }}>
                {move || if connected.get() { "Connected" } else { "Disconnected" }}
            </div>
        </header>

        // ── Intro / Context ──
        <div class="intro">
            <h2>"HVAC Temperature Control Simulation"</h2>
            <p>
                "This dashboard visualizes a "
                <strong>"PID controller"</strong>
                " managing a simulated room's heating system in real time. "
                "The controller's job is to drive the room temperature (currently starting at 5 \u{00B0}C) "
                "to a target of 22 \u{00B0}C, while the outside ambient temperature is 15 \u{00B0}C. "
                "A disturbance (window opening) occurs at t=15s, dropping the temperature by 2 \u{00B0}C."
            </p>
            <p>
                "A PID controller continuously computes a control signal from three terms: "
                <strong>"P"</strong>" (proportional \u{2014} reacts to current error), "
                <strong>"I"</strong>" (integral \u{2014} accumulates past error to eliminate steady-state offset), and "
                <strong>"D"</strong>" (derivative \u{2014} anticipates future error to damp oscillations)."
            </p>
        </div>

        // ── PID Formula ──
        <div class="pid-formula">
            <span class="eq">"output = "</span>
            <span class="p">"Kp \u{00B7} error"</span>
            <span class="eq">" + "</span>
            <span class="i">"Ki \u{00B7} \u{222B}error\u{00B7}dt"</span>
            <span class="eq">" + "</span>
            <span class="d">"Kd \u{00B7} d(error)/dt"</span>
        </div>

        // ── Live Metrics ──
        <div class="metrics">
            {move || {
                let data = pid_data.get();
                let latest = data.last();
                let (pv, sp, err, out) = match latest {
                    Some(d) => (
                        format!("{:.1} \u{00B0}C", d.process_value),
                        format!("{:.1} \u{00B0}C", d.setpoint),
                        format!("{:+.2} \u{00B0}C", d.error),
                        format!("{:.1}%", d.output),
                    ),
                    None => ("--".into(), "--".into(), "--".into(), "--".into()),
                };
                view! {
                    <div class="metric-card">
                        <span class="metric-label">"Process Value"</span>
                        <span class="metric-value">{pv}</span>
                        <span class="metric-sublabel">"Current room temperature"</span>
                    </div>
                    <div class="metric-card">
                        <span class="metric-label">"Setpoint"</span>
                        <span class="metric-value">{sp}</span>
                        <span class="metric-sublabel">"Target temperature"</span>
                    </div>
                    <div class="metric-card">
                        <span class="metric-label">"Error"</span>
                        <span class="metric-value">{err}</span>
                        <span class="metric-sublabel">"Setpoint minus process value"</span>
                    </div>
                    <div class="metric-card">
                        <span class="metric-label">"Output"</span>
                        <span class="metric-value">{out}</span>
                        <span class="metric-sublabel">"Heater power (-100 to +100)"</span>
                    </div>
                }
            }}
        </div>

        // ── Charts ──
        <div class="charts">
            <div class="chart-panel">
                <div class="chart-header">
                    <h2>"Process Value & Setpoint"</h2>
                    <span class="chart-hint">"Blue line should converge to dashed red line"</span>
                </div>
                <p class="chart-desc">
                    "Room temperature (blue) vs target (dashed red). "
                    "The orange error curve (right axis) shows how far off the controller is. "
                    "Watch for overshoot (blue exceeds red), settling time (how long until stable), "
                    "and disturbance recovery at t=15s."
                </p>
                <div class="chart-wrapper">
                    <canvas id="pv-chart"></canvas>
                </div>
            </div>
            <div class="chart-panel">
                <div class="chart-header">
                    <h2>"Control Output"</h2>
                    <span class="chart-hint">"What the controller tells the heater to do"</span>
                </div>
                <p class="chart-desc">
                    "The signal sent to the HVAC system. Positive = heating, negative = cooling. "
                    "Clamped to [-100%, +100%]. When pinned at a limit, the actuator is saturated "
                    "and anti-windup prevents the integral term from accumulating unbounded error."
                </p>
                <div class="chart-wrapper">
                    <canvas id="output-chart"></canvas>
                </div>
            </div>
            <div class="chart-panel">
                <div class="chart-header">
                    <h2>"PID Term Decomposition"</h2>
                    <span class="chart-hint">"Which term is doing the work?"</span>
                </div>
                <p class="chart-desc">
                    "Breaks the output into its three components. "
                    <strong>"P (blue)"</strong>" reacts to current error\u{2014}large early, shrinks near setpoint. "
                    <strong>"I (red)"</strong>" grows over time to eliminate steady-state offset. "
                    <strong>"D (green)"</strong>" damps oscillations by opposing rapid changes."
                </p>
                <div class="chart-wrapper">
                    <canvas id="pid-chart"></canvas>
                </div>
            </div>
        </div>

        // ── What to Look For ──
        <div class="what-to-look-for">
            <h3>"What to look for"</h3>
            <ul>
                <li><strong>"Initial ramp-up (0\u{2013}5s):"</strong>" Output saturates at 100% as the controller aggressively heats from 5 \u{00B0}C toward 22 \u{00B0}C. The P-term dominates."</li>
                <li><strong>"Settling (~5\u{2013}10s):"</strong>" Temperature converges to setpoint. The I-term accumulates to compensate for steady-state heat loss to ambient."</li>
                <li><strong>"Disturbance at t=15s:"</strong>" A simulated window opens, dropping temperature by 2 \u{00B0}C. Watch how quickly the controller detects and corrects the disturbance."</li>
                <li><strong>"Recovery (~15\u{2013}20s):"</strong>" The controller reacts to the disturbance. P-term spikes, I-term adjusts, and the system returns to setpoint."</li>
                <li><strong>"Steady state (~20s+):"</strong>" Temperature holds at setpoint. The I-term provides the constant offset needed to balance heat loss. P and D are near zero."</li>
            </ul>
        </div>
    }
}

/// Register a global JS function that creates/updates all charts.
/// Called once at startup. The function handles lazy chart creation.
#[cfg(feature = "hydrate")]
fn setup_chart_functions() {
    let js = r#"
window.__pidgeoneerUpdate = function(labels, pv, sp, error, output, pTerm, iTerm, dTerm) {
    if (typeof Chart === 'undefined') return;
    if (!window.__charts) window.__charts = {};

    var gridColor = 'rgba(255,255,255,0.06)';
    var tickColor = '#666';

    function ensure(id, cfg) {
        var el = document.getElementById(id);
        if (!el) return null;
        if (!window.__charts[id]) {
            window.__charts[id] = new Chart(el, cfg);
        }
        return window.__charts[id];
    }

    function upd(chart, lbl, datasets) {
        chart.data.labels = lbl;
        for (var i = 0; i < datasets.length; i++) {
            chart.data.datasets[i].data = datasets[i];
        }
        chart.update('none');
    }

    var baseScales = {
        x: {
            ticks: { color: tickColor, maxTicksLimit: 10 },
            grid: { color: gridColor },
            title: { display: true, text: 'Time (s)', color: tickColor }
        },
        y: {
            ticks: { color: tickColor },
            grid: { color: gridColor }
        }
    };

    // Chart 1: Process Value + Setpoint (left axis) and Error (right axis)
    var c1 = ensure('pv-chart', {
        type: 'line',
        data: {
            labels: [],
            datasets: [
                { label: 'Process Value', data: [], borderColor: '#3b82f6', borderWidth: 2, pointRadius: 0, fill: false, tension: 0.1 },
                { label: 'Setpoint', data: [], borderColor: '#ef4444', borderDash: [6, 3], borderWidth: 2, pointRadius: 0, fill: false },
                { label: 'Error', data: [], borderColor: '#f59e0b', borderWidth: 1.5, pointRadius: 0, fill: false, yAxisID: 'y1' }
            ]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            animation: false,
            interaction: { mode: 'index', intersect: false },
            plugins: {
                legend: { labels: { color: '#ccc', usePointStyle: true, pointStyle: 'line' } }
            },
            scales: {
                x: baseScales.x,
                y: {
                    ticks: { color: tickColor },
                    grid: { color: gridColor },
                    title: { display: true, text: 'Temperature (\u00B0C)', color: tickColor },
                    position: 'left'
                },
                y1: {
                    ticks: { color: tickColor },
                    grid: { drawOnChartArea: false },
                    title: { display: true, text: 'Error (\u00B0C)', color: tickColor },
                    position: 'right'
                }
            }
        }
    });
    if (c1) upd(c1, labels, [pv, sp, error]);

    // Chart 2: Control Output
    var c2 = ensure('output-chart', {
        type: 'line',
        data: {
            labels: [],
            datasets: [
                { label: 'Output', data: [], borderColor: '#22c55e', backgroundColor: 'rgba(34,197,94,0.08)', borderWidth: 2, pointRadius: 0, fill: true, tension: 0.1 }
            ]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            animation: false,
            interaction: { mode: 'index', intersect: false },
            plugins: {
                legend: { labels: { color: '#ccc', usePointStyle: true, pointStyle: 'line' } }
            },
            scales: {
                x: baseScales.x,
                y: {
                    ticks: { color: tickColor },
                    grid: { color: gridColor },
                    title: { display: true, text: 'Control Signal (%)', color: tickColor }
                }
            }
        }
    });
    if (c2) upd(c2, labels, [output]);

    // Chart 3: P, I, D terms
    var c3 = ensure('pid-chart', {
        type: 'line',
        data: {
            labels: [],
            datasets: [
                { label: 'P (proportional)', data: [], borderColor: '#3b82f6', borderWidth: 2, pointRadius: 0, fill: false, tension: 0.1 },
                { label: 'I (integral)', data: [], borderColor: '#ef4444', borderWidth: 2, pointRadius: 0, fill: false, tension: 0.1 },
                { label: 'D (derivative)', data: [], borderColor: '#22c55e', borderWidth: 2, pointRadius: 0, fill: false, tension: 0.1 }
            ]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            animation: false,
            interaction: { mode: 'index', intersect: false },
            plugins: {
                legend: { labels: { color: '#ccc', usePointStyle: true, pointStyle: 'line' } }
            },
            scales: {
                x: baseScales.x,
                y: {
                    ticks: { color: tickColor },
                    grid: { color: gridColor },
                    title: { display: true, text: 'Contribution', color: tickColor }
                }
            }
        }
    });
    if (c3) upd(c3, labels, [pTerm, iTerm, dTerm]);
};
"#;
    let _ = js_sys::eval(js);
}

/// Extract chart data from the PidControllerData buffer and call the JS update function.
#[cfg(feature = "hydrate")]
fn update_all_charts(data: &[PidControllerData]) {
    let start = data.len().saturating_sub(MAX_CHART_POINTS);
    let slice = &data[start..];

    // Compute relative time labels (seconds from first data point)
    let t0 = slice.first().map(|d| d.timestamp).unwrap_or(0);
    let labels: Vec<f64> = slice
        .iter()
        .map(|d| (d.timestamp.saturating_sub(t0)) as f64 / 1000.0)
        .collect();

    let pv: Vec<f64> = slice.iter().map(|d| d.process_value).collect();
    let sp: Vec<f64> = slice.iter().map(|d| d.setpoint).collect();
    let error: Vec<f64> = slice.iter().map(|d| d.error).collect();
    let output: Vec<f64> = slice.iter().map(|d| d.output).collect();
    let p_term: Vec<f64> = slice.iter().map(|d| d.p_term).collect();
    let i_term: Vec<f64> = slice.iter().map(|d| d.i_term).collect();
    let d_term: Vec<f64> = slice.iter().map(|d| d.d_term).collect();

    let labels_json = serde_json::to_string(&labels).unwrap_or_default();
    let pv_json = serde_json::to_string(&pv).unwrap_or_default();
    let sp_json = serde_json::to_string(&sp).unwrap_or_default();
    let error_json = serde_json::to_string(&error).unwrap_or_default();
    let output_json = serde_json::to_string(&output).unwrap_or_default();
    let p_json = serde_json::to_string(&p_term).unwrap_or_default();
    let i_json = serde_json::to_string(&i_term).unwrap_or_default();
    let d_json = serde_json::to_string(&d_term).unwrap_or_default();

    let js = format!(
        "window.__pidgeoneerUpdate({},{},{},{},{},{},{},{})",
        labels_json, pv_json, sp_json, error_json, output_json, p_json, i_json, d_json
    );
    let _ = js_sys::eval(&js);
}
