//! Shared CSS, nav bar, layout wrapper (Dracula theme).

pub const CSS: &str = r#"
:root {
    --bg: #282a36;
    --bg-dark: #21222c;
    --bg-light: #44475a;
    --fg: #f8f8f2;
    --fg-dim: #6272a4;
    --cyan: #8be9fd;
    --green: #50fa7b;
    --orange: #ffb86c;
    --pink: #ff79c6;
    --purple: #bd93f9;
    --red: #ff5555;
    --yellow: #f1fa8c;
}

* { margin: 0; padding: 0; box-sizing: border-box; }

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, monospace;
    background: var(--bg);
    color: var(--fg);
    line-height: 1.6;
}

a { color: var(--purple); text-decoration: none; }
a:hover { color: var(--pink); }

nav {
    background: var(--bg-dark);
    padding: 0.75rem 1.5rem;
    display: flex;
    align-items: center;
    gap: 2rem;
    border-bottom: 1px solid var(--bg-light);
}

nav .brand {
    font-size: 1.2rem;
    font-weight: bold;
    color: var(--purple);
}

nav .links { display: flex; gap: 1.5rem; }

nav .links a {
    color: var(--fg-dim);
    font-size: 0.9rem;
    padding: 0.25rem 0;
}

nav .links a:hover, nav .links a.active {
    color: var(--fg);
    border-bottom: 2px solid var(--purple);
}

.container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 1.5rem;
}

h1, h2, h3 { color: var(--fg); margin-bottom: 1rem; }
h1 { font-size: 1.5rem; }
h2 { font-size: 1.2rem; }

.card {
    background: var(--bg-dark);
    border: 1px solid var(--bg-light);
    border-radius: 6px;
    padding: 1.25rem;
    margin-bottom: 1rem;
}

.grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1rem;
}

.stat { text-align: center; padding: 1rem; }
.stat .value { font-size: 2rem; font-weight: bold; color: var(--purple); }
.stat .label { font-size: 0.85rem; color: var(--fg-dim); }

table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 0.5rem;
}

th, td {
    text-align: left;
    padding: 0.6rem 0.8rem;
    border-bottom: 1px solid var(--bg-light);
}

th { color: var(--fg-dim); font-size: 0.85rem; text-transform: uppercase; }

tr:hover { background: var(--bg-light); }

.badge {
    display: inline-block;
    padding: 0.15rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: bold;
}

.badge-green { background: var(--green); color: var(--bg); }
.badge-yellow { background: var(--yellow); color: var(--bg); }
.badge-red { background: var(--red); color: var(--bg); }
.badge-cyan { background: var(--cyan); color: var(--bg); }
.badge-purple { background: var(--purple); color: var(--bg); }
.badge-dim { background: var(--bg-light); color: var(--fg-dim); }

button, .btn {
    background: var(--purple);
    color: var(--bg);
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
    font-weight: bold;
}

button:hover, .btn:hover { background: var(--pink); }

button.danger { background: var(--red); }
button.danger:hover { background: #ff3333; }

button.sm { padding: 0.3rem 0.6rem; font-size: 0.75rem; }

input, select, textarea {
    background: var(--bg);
    border: 1px solid var(--bg-light);
    color: var(--fg);
    padding: 0.5rem;
    border-radius: 4px;
    font-size: 0.9rem;
    width: 100%;
}

input:focus, select:focus, textarea:focus {
    outline: none;
    border-color: var(--purple);
}

.form-group { margin-bottom: 0.75rem; }
.form-group label { display: block; color: var(--fg-dim); font-size: 0.85rem; margin-bottom: 0.25rem; }

.form-row {
    display: flex;
    gap: 1rem;
    align-items: end;
    flex-wrap: wrap;
}

.form-row .form-group { flex: 1; min-width: 150px; }

.toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
}

.toolbar .actions { display: flex; gap: 0.5rem; }

details.create-form {
    margin-bottom: 1rem;
}

details.create-form summary {
    list-style: none;
    cursor: pointer;
}

details.create-form summary::-webkit-details-marker { display: none; }

details.create-form[open] .card { margin-top: 0.5rem; }

.empty-state {
    text-align: center;
    padding: 3rem 1rem;
    color: var(--fg-dim);
}

.empty-state p { margin-bottom: 1rem; }

.url-cell {
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.htmx-indicator { display: none; }
.htmx-request .htmx-indicator { display: inline; }
.htmx-request button { opacity: 0.5; pointer-events: none; }

.flash {
    padding: 0.75rem 1rem;
    border-radius: 4px;
    margin-bottom: 1rem;
}
.flash-success { background: var(--green); color: var(--bg); }
.flash-error { background: var(--red); color: var(--bg); }
"#;

/// Wrap page content in the layout shell.
pub fn layout(title: &str, active: &str, content: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title} — StormStar</title>
    <style>{CSS}</style>
    <script src="https://unpkg.com/htmx.org@2.0.4"></script>
</head>
<body>
    <nav>
        <span class="brand">StormStar</span>
        <div class="links">
            <a href="/" {da}>Dashboard</a>
            <a href="/ui/repos" {ra}>Repositories</a>
            <a href="/ui/views" {va}>Content Views</a>
            <a href="/ui/envs" {ea}>Environments</a>
            <a href="/ui/hosts" {ha}>Hosts</a>
            <a href="/ui/errata" {xa}>Errata</a>
            <a href="/ui/keys" {ka}>Keys</a>
        </div>
    </nav>
    <div class="container">
        {content}
    </div>
</body>
</html>"#,
        title = title,
        CSS = CSS,
        content = content,
        da = if active == "dashboard" { r#"class="active""# } else { "" },
        ra = if active == "repos" { r#"class="active""# } else { "" },
        va = if active == "views" { r#"class="active""# } else { "" },
        ea = if active == "envs" { r#"class="active""# } else { "" },
        ha = if active == "hosts" { r#"class="active""# } else { "" },
        xa = if active == "errata" { r#"class="active""# } else { "" },
        ka = if active == "keys" { r#"class="active""# } else { "" },
    )
}
