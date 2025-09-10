import React from 'react';

async function logsLoad() {
  const response = await fetch("/portal/rest?op=/logs/get");
  const requests = await response.json();
  return requests;
}

export default function portlet_admin_logs() {
  const [logs, setlogs] = React.useState([]);

  async function logsRefresh() {
    setlogs(await logsLoad());
  }

  React.useEffect(() => {
    logsRefresh();
  }, []);

  const logsRows = [];
  for (var i = 0; i < logs.length; i++) {
    var setting = logs[i];
    logsRows.push(<tr>
      <td>{setting.log_timestamp}</td>
      <td>{setting.method}</td>
      <td>{setting.request}</td>
      <td>{setting.response}</td>
      <td>{setting.message}</td>
    </tr>);
  }

  return (
    <div class="row">
      <div class="col-12 col-lg-12">
        <div class="card">
          <div class="card-header">
            <button type="button" class="btn btn-primary" title="refresh" onClick={logsRefresh}><i class="align-middle" data-feather="refresh-ccw"></i></button>
          </div>
          <div class="card-body">
            <table class="table table-hover my-0">
              <thead>
                <tr>
                  <th>time</th>
                  <th>method</th>
                  <th>request</th>
                  <th>response</th>
                  <th>message</th>
                </tr>
              </thead>
              <tbody>
                {logsRows}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}