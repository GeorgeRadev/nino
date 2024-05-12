import React from 'react';

async function requestsLoad() {
  const response = await fetch("/portal_rest?op=/requests/get");
  const requests = await response.json();
  return requests;
}

export default function portlet_admin_requests() {
  const [requests, setRequests] = React.useState([]);
  const [selectIx, setSelectIx] = React.useState(-1);

  async function requestRefresh() {
    setRequests(await requestsLoad());
    setSelectIx(-1);
    setTimeout(feather.replace, 20);
  }

  React.useEffect(() => {
    requestRefresh();
  }, []);

  function onRowClick(e) {
    setSelectIx(e.target.parentElement.dataset.index);
  }

  const requestRows = [];
  for (var i = 0; i < requests.length; i++) {
    var request = requests[i];
    requestRows.push(<tr class={(i == selectIx) ? "table-primary" : ""} data-index={i} onClick={onRowClick}>
      <td>{request.request_path}</td>
      <td>{request.response_name}</td>
      <td><i class="align-middle" data-feather={request.redirect_flag == 'true' ? 'check-square' : 'minus'}></i></td>
      <td><i class="align-middle" data-feather={request.authorize_flag == 'true' ? 'check-square' : 'minus'}></i></td>
    </tr>);
  }

  return (
    <div class="row">
      <div class="col-12 col-lg-12">
        <div class="card">
          <div class="card-header">
            <button type="button" class="btn btn-primary" title="refresh" onClick={requestRefresh}><i class="align-middle" data-feather="refresh-ccw"></i></button>
          </div>
          <div class="card-body">
            <table class="table table-hover my-0">
              <thead>
                <tr>
                  <th>request path</th>
                  <th>response name</th>
                  <th>redirect</th>
                  <th>authorize</th>
                </tr>
              </thead>
              <tbody>
                {requestRows}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}