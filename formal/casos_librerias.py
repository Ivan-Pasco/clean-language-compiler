"""Casos de sintaxis de las gramáticas de librería: ejemplos de la ley.

Un caso no copia el código ni fija números de línea: nombra el capítulo,
una línea ancla que aparece en él exactamente una vez, y el tramo de
líneas alrededor del ancla (`desde` y `hasta`, relativos a ella), así
que sobrevive a que el capítulo se enmiende por encima.  `dedenta`
quita ese número de tabs del principio de cada línea: un bloque que en
el ejemplo está dentro de otro.  `inicio` es la regla de inicio de la
gramática de la librería (su `start` o una de su línea `// %start`).

Solo entran ejemplos que la gramática de la librería lee tal como están
escritos; los que discrepan de ella son un asunto de la ley, no casos.
"""
from __future__ import annotations

from componer import LEY

LIB = "architecture-boundaries/framework/libraries/"


def caso(id, libreria, archivo, ancla, desde, hasta, inicio,
         sintaxis="valida", dedenta=0):
    return dict(id=id, libreria=libreria, archivo=LIB + archivo, ancla=ancla,
                desde=desde, hasta=hasta, inicio=inicio, sintaxis=sintaxis,
                dedenta=dedenta)


CASOS = [
    caso("LBSG-01.e1", "09-libraries-specification", "08-server.md",
         '\tGET "/api/secure" [admin] cache(60) middleware(RateLimit, Audit) :',
         -19, 1, "block_call"),
    caso("LBSG-02.e1", "09-libraries-specification", "00-libraries.md",
         'host interface ui version "0.1.0":', -2, 10, "host_bridge_file"),
    caso("CLTG-03.e1", "03-client", "03-client.md",
         '\t\tfields = ["email", "password", "name"]', -2, 2, "start",
         dedenta=1),
    caso("DATG-01.e1", "04-data", "04-data.md",
         '\t\tuserId as "user_id" required', -5, 9, "data_block"),
    caso("JOBG-02.e1", "05-jobs", "05-jobs.md",
         '\t\temail.send(email, "Welcome!", "<p>Welcome to our service.</p>", '
         '"Welcome to our service.")', -9, 0, "start"),
    caso("LOCG-01.e1", "06-locale", "06-locale.md",
         '\tdetection = "header"', -3, 1, "locale_block"),
    caso("MCPG-01.e1", "07-mcp", "07-mcp.md",
         '\tdescription: "Authenticated MCP server"', -2, 4, "mcp_block"),
    caso("SRVG-01.e1", "08-server", "08-server.md",
         '\tGET "/api/secure" [admin] cache(60) middleware(RateLimit, Audit) :',
         -19, 1, "start"),
    caso("SRVG-04.e1", "08-server", "08-server.md",
         '\tGET "/api/reports/live" [admin, analyst] cache("no-store") '
         'middleware(Audit) :', -5, 2, "start"),
    caso("UIG-01.e1", "10-ui", "10-ui.md",
         '\t\t\t<img src="{inputs.avatar}" alt="{inputs.name}" />', -7, 2,
         "component_block"),
    # Ejemplos que la reconciliación de la prosa de librerías con sus
    # gramáticas dejó válidos (FS-03, FS-04).
    caso("SRVG-01.e2", "08-server", "08-server.md", '\tGET "/api/users/:id" :', -14, 5, "start"),
    caso("SRVG-01.e3", "08-server", "08-server.md", '\tSTREAM "/api/generate/:jobId":', -1, 15, "start"),
    caso("SRVG-03.e1", "08-server", "08-server.md", '\tPOST "/api/avatar" [user]:', -1, 7, "start"),
    caso("SRVG-07.e1", "08-server", "08-server.md", '\tPOST "/auth/magic-link":', -1, 14, "start"),
    caso("MCPG-02.e1", "07-mcp", "07-mcp.md", '\tdescription: "Tools for AI agents to read and manage blog content"', -2, 144, "mcp_block"),
    caso("MCPG-06.e1", "07-mcp", "07-mcp.md", '\t\tauth: bearer env.get("SEARCH_TOKEN")', -13, 1, "mcp_clients_block"),
    caso("DATG-05.e1", "04-data", "04-data.md", '\t\tlist<User> findRecentByStatus(string status, integer resultLimit)', -34, 6, "data_block"),
    caso("DATG-07.e1", "04-data", "04-data.md", '\t\tage >= 18', -3, 3, "query_expression"),
    caso("DATG-07.e2", "04-data", "04-data.md", '\t\temail == "alice@example.com"', -2, 0, "query_expression"),
    caso("DATG-07.e3", "04-data", "04-data.md", 'User.findOrFail:', 0, 2, "query_expression"),
    caso("DATG-07.e4", "04-data", "04-data.md", 'User.count:', 0, 2, "query_expression"),
    caso("DATG-07.e5", "04-data", "04-data.md", '\t\temail == candidateEmail', -2, 0, "query_expression"),
    caso("DATG-07.e6", "04-data", "04-data.md", '\tselect:', -1, 4, "query_expression"),
    caso("DATG-07.e7", "04-data", "04-data.md", '\toffset: (page - 1) * perPage', -6, 0, "query_expression"),
    caso("DATG-07.e8", "04-data", "04-data.md", 'User.paginate:', 0, 4, "query_expression"),
    caso("DATG-07.e9", "04-data", "04-data.md", '\tafter: previousCursor', -4, 1, "query_expression"),
    caso("DATG-07.e10", "04-data", "04-data.md", '\t\tpublished == true', -5, 3, "query_expression"),
    caso("DATG-07.e11", "04-data", "04-data.md", '\t\ttenantId == currentTenantId', -4, 1, "query_expression"),
    caso("DATG-07.e12", "04-data", "04-data.md", '\t\tUserRole.userId == currentUserId', -4, 0, "query_expression"),
    caso("DATG-09.e1", "04-data", "04-data.md", '\tProfile profile = Profile(', -9, 4, "transaction_block"),
    caso("DATG-10.e1", "04-data", "04-data.md", '\tsql: "SELECT id, email, COUNT(*) AS post_count FROM users JOIN posts ON ..."', -1, 1, "raw_query_block"),
    caso("DATG-10.e2", "04-data", "04-data.md", '\tsql: "SELECT COUNT(*) AS total FROM users"', -1, 0, "raw_query_block"),
    caso("JOBG-01.e1", "05-jobs", "05-jobs.md", '\t\temail.send(email, "Welcome!", "<p>Welcome to our service.</p>", "Welcome to our service.")', -9, 10, "start"),
    caso("JOBG-01.e2", "05-jobs", "05-jobs.md", '\t\t\tqueue.enqueue("sendWelcomeEmail", users[idx].email)', -23, 1, "start"),
    caso("JOBG-02.e2", "05-jobs", "05-jobs.md", '\t\tjob.succeed("\\{\\"paymentId\\": \\"{paymentId}\\", \\"charged\\": true}")', -15, 0, "start"),
    caso("JOBG-04.e1", "05-jobs", "05-jobs.md", '\t\tDatabase.deleteAll(Token.data.findExpiredBefore(now()))', -9, 0, "start"),
    caso("SRVG-01.e4", "08-server", "04-data.md", '\t\treturn json(json.textToData("\\{\\"id\\": {user.id!}, \\"email\\": \\"{user.email}\\"}"))', -7, 4, "start"),
    caso("SRVG-01.e5", "08-server", "05-jobs.md", '\t\treturn json(json.textToData("\\{\\"ok\\": true, \\"userId\\": {u.id!}}"))', -9, 0, "start"),
    caso("SRVG-01.e6", "08-server", "06-locale.md", '\t\tstring countMsg = tc("products.count", products.length(), "\\{}")', -3, 5, "start"),
    caso("SRVG-01.e7", "08-server", "01-auth.md", '\tPOST "/auth/login" :', -1, 11, "start"),
    caso("SRVG-03.e2", "08-server", "01-auth.md", '\tPOST "/admin/publish" [editor, admin]:', -1, 1, "start"),
    caso("SRVG-01.e8", "08-server", "09-storage.md", '\tPOST "/api/v1/reports/tarball-upload" :', -1, 13, "start"),
    caso("AGTG-01.e1", "11-agent", "11-agent.md", '\tsystem: "You are a helpful customer support agent."', -3, 14, "agent_block"),
    caso("CLTG-01.e1", "03-client", "03-client.md", '\tload "/api/posts" as "posts":', 0, 3, "start", dedenta=1),
    caso("CLTG-01.e2", "03-client", "03-client.md", '\tload "/api/products/{inputs.productId}" as "product":', 0, 1, "start", dedenta=1),
    caso("CLTG-05.e1", "03-client", "03-client.md", '\tsend "/api/posts/{inputs.postId}/like" as "like":', 0, 3, "start", dedenta=1),
    caso("UIG-01.e2", "10-ui", "03-client.md", 'component tag="user-list" client="on":', 0, 26, "component_block"),
    caso("UIG-01.e3", "10-ui", "03-client.md", 'component tag="chat-window" client="on":', 0, 29, "component_block"),
    caso("UIG-01.e4", "10-ui", "03-client.md", 'component tag="generation-log" client="on":', 0, 35, "component_block"),
    caso("UIG-01.e5", "10-ui", "10-ui.md", '// app/ui/web/components/UserCard.cln', 1, 14, "component_block"),
    caso("UIG-01.e6", "10-ui", "10-ui.md", 'component tag="live-editor" client="on":', 0, 3, "component_block"),
    caso("UIG-01.e7", "10-ui", "10-ui.md", 'component tag="user-card" css="/css/shared/cards.css":', 0, 2, "component_block"),
    caso("UIG-01.e8", "10-ui", "10-ui.md", 'component tag="user-form":', 0, 6, "component_block"),
    caso("UIG-01.e9", "10-ui", "10-ui.md", 'component tag="comment-form" client="on":', 0, 25, "component_block"),
    caso("UIG-02.e1", "10-ui", "10-ui.md", '\t\t\t<nav class="navbar">', -1, 3, "html_block", dedenta=2),
    caso("UIG-02.e2", "10-ui", "10-ui.md", '\t\thtml var="header":', 0, 3, "html_block", dedenta=2),
    caso("UIG-02.e3", "10-ui", "10-ui.md", '\t<h3 class="title">{this.title}</h3>', -1, 1, "html_block"),
    caso("UIG-01.e10", "10-ui", "12-ui-runtime.md", 'component tag="file-tree":', 0, 7, "component_block"),
    caso("UIG-01.e11", "10-ui", "12-ui-runtime.md", 'component tag="app-shell":', 0, 20, "component_block"),
    caso("UIG-01.e12", "10-ui", "12-ui-runtime.md", 'component tag="confirm-modal":', 0, 12, "component_block"),
    caso("UIG-01.e13", "10-ui", "12-ui-runtime.md", 'component tag="lazy-image":', 0, 15, "component_block"),
    caso("UIG-01.e14", "10-ui", "12-ui-runtime.md", 'component tag="user-list" client="visible":   // client= triggers client_init emission for this component', 0, 9, "component_block"),
    caso("UIG-02.e4", "10-ui", "12-ui-runtime.md", '\t\t\t\t<p>Could not load user information.</p>', -2, 2, "html_block", dedenta=2),
    caso("LBSG-02.e2", "09-libraries-specification", "12-ui-runtime.md", '\thost function toggleClass(selector: string, className: string) returns integer', -3, 1, "host_bridge_file"),
    caso("CNVG-01.e1", "02-canvas", "canvas/00-canvas.md", 'canvasScene width=800 height=600 fps=60 id="main":', 0, 2, "start"),
    caso("CNVG-01.e2", "02-canvas", "canvas/00-canvas.md", '\tclass Square can CanvasDraw', -2, 13, "start"),
    caso("CNVG-01.e3", "02-canvas", "canvas/00-canvas.md", '\tclass Background can CanvasDraw', -15, 47, "start"),
    caso("CNVG-01.e4", "02-canvas", "canvas/00-canvas.md", '\tgradient "sky" linear x1=0 y1=0 x2=0 y2=600:', -1, 11, "start"),
    caso("CNVG-01.e5", "02-canvas", "canvas/00-canvas.md", '\tgradient "glow" radial cx=400 cy=300 r=200:', -1, 11, "start"),
    caso("CNVG-01.e6", "02-canvas", "canvas/14-canvas-runtime.md", 'canvasScene width=800 height=600 id="game":', 0, 2, "start"),
    caso("CNVG-05.e1", "02-canvas", "canvas/00-canvas.md", '\tclass Planet can CanvasDraw', -1, 25, "start"),
    caso("CNVG-05.e2", "02-canvas", "canvas/00-canvas.md", '\tclass MusicPlayer can CanvasDraw, CanvasLifecycle', -1, 17, "start"),
    caso("CNVG-05.e3", "02-canvas", "canvas/14-canvas-runtime.md", '\tclass Slider can CanvasDraw', -1, 19, "start"),
    caso("CNVG-07.e1", "02-canvas", "canvas/14-canvas-runtime.md", '\tcustomEase "snap" x1=0.68 y1=-0.55 x2=0.265 y2=1.55', -1, 8, "start"),
    caso("CNVG-08.e1", "02-canvas", "canvas/13-canvas-animation.md", '\tclass MenuBackground can CanvasDraw', -11, 70, "start"),
    caso("CNVG-11.e1", "02-canvas", "canvas/14-canvas-runtime.md", '\tclass Background can CanvasDraw', -48, 120, "start"),
    caso("CNVG-13.e1", "02-canvas", "canvas/00-canvas.md", '\tclass SolarSystem can CanvasDraw', -1, 18, "start"),
    caso("CNVG-13.e2", "02-canvas", "canvas/00-canvas.md", '\tclass Enemy can CanvasDraw', -5, 15, "start"),
    caso("CNVG-13.e3", "02-canvas", "canvas/00-canvas.md", '\tclass DebugOverlay can CanvasDraw', -1, 11, "start"),
    caso("CNVG-13.e4", "02-canvas", "canvas/13-canvas-animation.md", '\tclass Button can CanvasDraw', -1, 15, "start"),
    caso("CNVG-13.e5", "02-canvas", "canvas/13-canvas-animation.md", '\tclass PathVisualizer can CanvasDraw', -1, 7, "start"),
    caso("CNVG-14.e1", "02-canvas", "canvas/13-canvas-animation.md", '\tclass Controller can CanvasDraw', -4, 15, "start"),
    caso("CNVG-14.e2", "02-canvas", "canvas/13-canvas-animation.md", '\tclass MenuController can CanvasDraw', -4, 17, "start"),
    caso("CNVG-14.e3", "02-canvas", "canvas/13-canvas-animation.md", '\tclass Hero can CanvasDraw', -4, 17, "start"),
    caso("CNVG-14.e4", "02-canvas", "canvas/13-canvas-animation.md", '\tclass Fireworks can CanvasDraw', -1, 11, "start"),
    caso("CNVG-14.e5", "02-canvas", "canvas/14-canvas-runtime.md", '\t\tboolean tookDamage = false', -2, 14, "start"),
]


def codigo_de(c) -> str:
    """El tramo del capítulo que el caso nombra, leído hoy."""
    lineas = (LEY / c["archivo"]).read_text(encoding="utf-8").split("\n")
    donde = [i for i, l in enumerate(lineas) if l == c["ancla"]]
    if len(donde) != 1:
        raise LookupError(f"{c['id']}: el ancla aparece {len(donde)} veces "
                          f"en {c['archivo']}")
    i = donde[0]
    trozo = lineas[i + c["desde"]:i + c["hasta"] + 1]
    n = c["dedenta"]
    if n:
        trozo = [l[n:] if l.startswith("\t" * n) else l for l in trozo]
    return "\n".join(trozo)
