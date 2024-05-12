import React from 'react';

async function requestsLoad() {
  const response = await fetch("/portal_rest?op=/requests/get");
  const requests = await response.json();
  return requests;
}

export default function portlet_admin_requests() {
  const [dialogVisible, setDialogVisible] = React.useState(false);
  const [requests, setRequests] = React.useState([]);

  async function requestRefresh() {
    setRequests(await requestsLoad());
    setTimeout(feather.replace, 20);
  }

  React.useEffect(() => {
    requestRefresh();
  }, []);

  function requestAdd() {
    setDialogVisible(true);
  }
  function requestEdit() { }
  function requestDelete() { }

  function dialogOk() {
    dialogClose();
  }
  function dialogClose() {
    setDialogVisible(false);
  }

  const requestRows = [];
  for (var request of requests) {
    requestRows.push(<tr>
      <td>{request.request_path}</td>
      <td>{request.response_name}</td>
      <td><i class="align-middle" data-feather={request.redirect_flag == 'true' ? 'check-square' : 'minus'}></i></td>
      <td><i class="align-middle" data-feather={request.authorize_flag == 'true' ? 'check-square' : 'minus'}></i></td>
    </tr>);
  }

  return (
    <>
      <div class="row">
        <div class="col-12 col-lg-12">
          <div class="card">
            <div class="card-header">
              <button type="button" class="btn btn-primary" title="refresh" onClick={requestRefresh}><i class="align-middle" data-feather="refresh-ccw"></i></button>
              &nbsp;&nbsp;&nbsp;
              <button type="button" class="btn btn-success" title="add request" onClick={requestAdd}><i class="align-middle" data-feather="plus"></i></button>
              &nbsp;
              <button type="button" class="btn btn-success" title="edit request" onClick={requestEdit}><i class="align-middle" data-feather="edit"></i></button>
              &nbsp;&nbsp;&nbsp;
              <button type="button" class="btn btn-danger" title="delete request" onClick={requestDelete}><i class="align-middle" data-feather="delete"></i></button>
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

        <div className="portlet-dialog-background" style={{ display: (dialogVisible) ? "block" : "none" }}>
          <div className="portlet-dialog-content">
            <div class="row">
              <div class="col-12 col-lg-12">
                <div class="card">
                  <div class="card-header">
                    <h5 class="card-title">Add New Request</h5>
                  </div>
                  <div class="card-body">
                    <button onClick={() => dialogClose()}>close</button>
                    <button onClick={() => dialogOk()}>ok</button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </>
  );
}