import React from 'react';

async function requestsLoad() {
  const response = await fetch("/portal_rest?op=/requests/get");
  const requests = await response.json();
  return requests;
}

export default function portlet_admin_requests() {
  const [requests, setRequests] = React.useState([]);
  const [selectIx, setSelectIx] = React.useState(-1);
  const [dialogVisible, setDialogVisible] = React.useState(false);
  const [responseDetails, setResponseDetails] = React.useState({});

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

  async function fetchResponseDetails() {
    if (selectIx >= 0 && selectIx < requests.length) {
      const response = await fetch("/portal_rest?op=/responses/detail&name=" + requests[selectIx].response_name);
      const details = await response.json();
      setResponseDetails(details);
      document.getElementById('requests_jsqlx_code').textContent = details.response_content;
      document.getElementById('requests_transpiled_code').textContent = details.javascript;
      document.getElementById('requests_jsqlx_code').style.display = "block";
      document.getElementById('requests_transpiled_code').style.display = "none";
      setDialogVisible(true);
    }
  }

  function dialogClose() {
    setDialogVisible(false);
  }
  function onRowClick(e) {
    if (e.target.parentElement.dataset.index == selectIx) {
      fetchResponseDetails();
    } else {
      setSelectIx(e.target.parentElement.dataset.index);
    }
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
    <>
      <div class="row">
        <div class="col-12 col-lg-12">
          <div class="card">
            <div class="card-header">
              <button type="button" class="btn btn-primary" title="refresh" onClick={requestRefresh}><i class="align-middle" data-feather="refresh-ccw"></i></button>
              &nbsp;&nbsp;&nbsp;
              <button type="button" class="btn btn-success" title="add response" onClick={responseDetails}><i class="align-middle" data-feather="file-text"></i></button>
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

      <div className="portlet-dialog-background" style={{ display: (dialogVisible) ? "block" : "none" }}>
        <div className="portlet-dialog-content">
          <div class="col-12 col-lg-12">
            <div class="card-header">
              <table width="100%">
                <tr>
                  <td><h5 class="card-title">Response Details</h5></td>
                  <td align="right"><button class="btn btn-primary" onClick={() => dialogClose()}>X</button></td>
                </tr>
              </table>
            </div>
            <div class="card-body">
              <table>
                <tr>
                  <td>response name:&nbsp;&nbsp;</td>
                  <td>{responseDetails['response_name']}</td>
                </tr>
                <tr>
                  <td>mime_type:</td>
                  <td>{responseDetails['response_mime_type']}</td>
                </tr>
                <tr>
                  <td>execute:</td>
                  <td>{responseDetails['execute_flag']}</td>
                </tr>
                <tr>
                  <td>transpile:</td>
                  <td>{responseDetails['transpile_flag']}</td>
                </tr>
                <tr>
                  <td>code: </td>
                  <td>
                    <button class="btn btn-primary" onClick={() => {
                      document.getElementById('requests_jsqlx_code').style.display = "block";
                      document.getElementById('requests_transpiled_code').style.display = "none";
                    }}>jsqlx</button>
                    &nbsp;
                    <button class="btn btn-primary" onClick={() => {
                      document.getElementById('requests_jsqlx_code').style.display = "none";
                      document.getElementById('requests_transpiled_code').style.display = "block";
                    }}>transpiled js</button>
                  </td>
                </tr>
              </table>
              <textarea class="form-control" style="font-family: Courier;" id="requests_jsqlx_code" rows="20"></textarea>
              <textarea class="form-control" style="font-family: Courier;" id="requests_transpiled_code" rows="20"></textarea>
            </div>
          </div>
        </div>
      </div>
    </>
  );
}