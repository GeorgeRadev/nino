const page = `<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8" />
  <title>Portal</title>
</head>

<body>
  <script type="importmap">
  {
      "imports": {
        "preact": "https://esm.sh/preact@10.17.1",
        "preact/": "https://esm.sh/preact@10.17.1/",
        "react": "https://esm.sh/preact@10.17.1/compat",
        "react-dom": "https://esm.sh/preact@10.17.1/compat"
      }
  }
  </script>

  <hr/>
  <div id="about"></div>
  <hr/>
  <div id="counter"></div>
  <hr/>
  
  <script type="module">
    import ReactDOM from 'react-dom';
    import about from './portlet_about.js';
    import counter from './portlet_counter.js';

    ReactDOM.render(
      ReactDOM.createElement(about, null), 
      document.getElementById('about'));
    ReactDOM.render(
      ReactDOM.createElement(counter, null), 
      document.getElementById('counter'));  
  </script>
</body>

</html>`;

export default async function portal(request, response) {
  debugger;
  await response.send(page);
}