import React from 'react';

async function usersLoad() {
  const response = await fetch("/portal_rest?op=/settings/get");
  const requests = await response.json();
  return requests;
}

export default function portlet_admin_settings() {
  const [settings, setSettings] = React.useState([]);
  const [selectIx, setSelectIx] = React.useState(-1);

  async function settingsRefresh() {
    setSettings(await usersLoad());
    setSelectIx(-1);
    setTimeout(feather.replace, 20);
  }

  React.useEffect(() => {
    settingsRefresh();
  }, []);

  function onRowClick(e) {
    setSelectIx(e.target.parentElement.dataset.index);
  }

  const settingsRows = [];
  for (var i = 0; i < settings.length; i++) {
    var setting = settings[i];
    settingsRows.push(<tr class={(i == selectIx) ? "table-primary" : ""} data-index={i} onClick={onRowClick}>
      <td>{setting.setting_key}</td>
      <td>{setting.setting_value}</td>
    </tr>);
  }

  return (
    <div class="row">
      <div class="col-12 col-lg-12">
        <div class="card">
          <div class="card-header">
            <button type="button" class="btn btn-primary" title="refresh" onClick={settingsRefresh}><i class="align-middle" data-feather="refresh-ccw"></i></button>
          </div>
          <div class="card-body">
            <table class="table table-hover my-0">
              <thead>
                <tr>
                  <th>setting</th>
                  <th>value</th>
                </tr>
              </thead>
              <tbody>
                {settingsRows}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}