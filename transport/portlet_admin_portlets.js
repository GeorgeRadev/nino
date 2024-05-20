import React from 'react';

async function portletsLoad() {
  const response = await fetch("/portal_rest?op=/portlets/get");
  const requests = await response.json();
  return requests;
}

export default function portlet_admin_portlets() {
  const [portlets, setPortlets] = React.useState([]);
  const [selectIx, setSelectIx] = React.useState(-1);

  async function portletsRefresh() {
    setPortlets(await portletsLoad());
    setSelectIx(-1);
    setTimeout(feather.replace, 20);
  }

  React.useEffect(() => {
    portletsRefresh();
  }, []);

  function onRowClick(e) {
    setSelectIx(e.target.parentElement.dataset.index);
  }

  const portletRows = [];
  var role_previous = "";
  for (var i = 0; i < portlets.length; i++) {
    var portlet = portlets[i];
    portletRows.push(<tr class={(i == selectIx) ? "table-primary" : ""} data-index={i} onClick={onRowClick}>
      <td>{(role_previous != portlet.user_role) ? portlet.user_role : ""}</td>
      <td>{portlet.portlet_menu}</td>
      <td>{portlet.portlet_index}</td>
      <td>{portlet.portlet_icon}</td>
      <td>{portlet.portlet_name}</td>
    </tr>);
    role_previous = portlet.user_role;
  }

  return (
    <div class="row">
      <div class="col-12 col-lg-12">
        <div class="card">
          <div class="card-header">
            <button type="button" class="btn btn-primary" title="refresh" onClick={portletsRefresh}><i class="align-middle" data-feather="refresh-ccw"></i></button>
          </div>
          <div class="card-body">
            <table class="table table-hover my-0">
              <thead>
                <tr>
                  <th>role</th>
                  <th>menu</th>
                  <th>index</th>
                  <th>icon</th>
                  <th>portlet</th>
                </tr>
              </thead>
              <tbody>
                {portletRows}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}