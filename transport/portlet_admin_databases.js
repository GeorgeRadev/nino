import React from 'react';

async function databasesLoad() {
  const response = await fetch("/portal_rest?op=/databases/get");
  const requests = await response.json();
  return requests;
}

export default function portlet_admin_databases() {
  const [dbs, setDBs] = React.useState([]);
  const [selectIx, setSelectIx] = React.useState(-1);

  async function databasesRefresh() {
    setDBs(await databasesLoad());
    setSelectIx(-1);
    setTimeout(feather.replace, 20);
  }

  React.useEffect(() => {
    databasesRefresh();
  }, []);

  function onRowClick(e) {
    setSelectIx(e.target.parentElement.dataset.index);
  }

  const databasesRows = [];
  for (var i = 0; i < dbs.length; i++) {
    var db = dbs[i];
    databasesRows.push(<tr class={(i == selectIx) ? "table-primary" : ""} data-index={i} onClick={onRowClick}>
      <td>{db.db_alias}</td>
      <td>{db.db_type}</td>
      <td>{db.db_connection_string}</td>
    </tr>);
  }

  return (
    <div class="row">
      <div class="col-12 col-lg-12">
        <div class="card">
          <div class="card-header">
            <button type="button" class="btn btn-primary" title="refresh" onClick={databasesRefresh}><i class="align-middle" data-feather="refresh-ccw"></i></button>
          </div>
          <div class="card-body">
            <table class="table table-hover my-0">
              <thead>
                <tr>
                  <th>alias</th>
                  <th>type</th>
                  <th>connection string</th>
                </tr>
              </thead>
              <tbody>
                {databasesRows}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}