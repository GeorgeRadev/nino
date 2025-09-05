import React from 'react';

async function usersLoad() {
  const response = await fetch("/portal/rest?op=/users/get");
  const requests = await response.json();
  return requests;
}

export default function portlet_admin_roles() {
  const [users, setUsers] = React.useState([]);
  const [selectIx, setSelectIx] = React.useState(-1);

  async function requestRefresh() {
    setUsers(await usersLoad());
    setSelectIx(-1);
    setTimeout(feather.replace, 20);
  }

  React.useEffect(() => {
    requestRefresh();
  }, []);

  function onRowClick(e) {
    setSelectIx(e.target.parentElement.dataset.index);
  }

  const userRows = [];
  var user_previous = "";
  for (var i = 0; i < users.length; i++) {
    var user = users[i];
    userRows.push(<tr class={(i == selectIx) ? "table-primary" : ""} data-index={i} onClick={onRowClick}>
      <td>{(user_previous != user.user_name) ? user.user_name : ""}</td>
      <td>{user.user_role}</td>
    </tr>);
    user_previous = user.user_name;
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
                  <th>user name</th>
                  <th>role</th>
                </tr>
              </thead>
              <tbody>
                {userRows}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}