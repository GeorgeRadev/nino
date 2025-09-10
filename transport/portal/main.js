import log from "_log";
import nino from "_nino";

export default async function portal(request) {
  await nino.assertRole(request, 'portal');

  const portlet_menu = await nino.getPortletMenu(request);
  // get portlet name
  var currentPortletName = (request.url || "").split('=')[1] || "";
  if (currentPortletName.length > 0 && currentPortletName.charAt(0) === '/') {
    currentPortletName = currentPortletName.substring(1);
  }
  // get first portlet as home when not found
  var currentPortlet;
  if (currentPortletName) {
    currentPortlet = portlet_menu[currentPortletName];
  }
  if (!currentPortlet) {
    for (var path in portlet_menu) {
      currentPortlet = portlet_menu[path];
      break;
    }
  }
  const portlet_last_token = currentPortlet['path'].split("/").pop();
  const menujson = JSON.stringify(portlet_menu);

  return `<!DOCTYPE html>
<html lang="en">

<head>
  <meta charset="utf-8">
  <meta http-equiv="X-UA-Compatible" content="IE=edge">
  <meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no">
  <meta name="description" content="Nino Portal">
  <link rel="shortcut icon" href="favicon.ico" />
  <base href="/">

  <title>Nino portal</title>

  <link href="/portal/portal.css" rel="stylesheet">
  <script type="text/javascript" src="/portal/portal.js"></script>
  <script type="importmap">
  {
      "imports": {
        "preact": "https://esm.sh/preact@10.27.1",
        "preact/": "https://esm.sh/preact@10.27.1/",
        "react": "https://esm.sh/preact@10.27.1/compat",
        "react-dom": "https://esm.sh/preact@10.27.1/compat"
      }
  }
  </script>
</head>

<body>
<div class="wrapper">
<nav id="sidebar" class="sidebar js-sidebar">
  <div class="sidebar-content js-simplebar">
    <a class="sidebar-brand" href="index.html">
      <span class="align-middle">NINO Portal</span>
    </a>
    <ul class="sidebar-nav" id="portlet_menu"></ul>
  </div>
</nav>

<div class="main">
  <nav class="navbar navbar-expand navbar-light navbar-bg">
    <a class="sidebar-toggle js-sidebar-toggle">
      <i class="hamburger align-self-center"></i>
    </a>

    <div class="navbar-collapse collapse">
      <h1 class="h3" style="margin-top: 0.5rem;" id="portlet-title">${portlet_last_token}</h1>
    </div>
  </nav>

  <main class="content">
    <div class="container-fluid p-0">
      <div id="portlet"></div>
    </div>
  </main>

  <script type="module">
    import ReactDOM from 'react-dom';
    import portlet_menu from '/portal/menu.js';
    import portlet      from '/${currentPortlet['portlet']}';

    window.portlet_menu = ${menujson};
    // render menu
    ReactDOM.render(
      ReactDOM.createElement(portlet_menu, null), 
      document.getElementById('portlet_menu'));
    // render portlet
    ReactDOM.render(
      ReactDOM.createElement(portlet, null), 
      document.getElementById('portlet'));
  </script>

  <footer class="footer">
    <div class="container-fluid">
      <div class="row text-muted">
        <div class="col-6 text-start">
          <p class="mb-0">
            <a class="text-muted" href="/" target="_blank"><strong>Nino - Portal</strong></a>
          </p>
        </div>
        <div class="col-6 text-end">
          <ul class="list-inline">
            <li class="list-inline-item">
              <a class="text-muted" href="/" target="_blank">Support</a>
            </li>
          </ul>
        </div>
      </div>
    </div>
  </footer>
</div>
</div>

</body>
</html>`;
}