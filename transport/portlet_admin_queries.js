import React from 'react';

export default function portlet_admin_databases() {
  const [aliases, setAliases] = React.useState([]);
  const [queryResult, setQueryResult] = React.useState([]);

  async function aliasesRefresh() {
    const response = await fetch("/portal_rest?op=/databases/get");
    const aliases = await response.json();
    setAliases(aliases);
  }

  async function executeQuery() {
    const response = await fetch("/portal_rest?op=/databases/query&" + new URLSearchParams({
      alias: document.getElementById("nino_database_alias").value,
      query: document.getElementById("nino_database_query").value,
    }));
    const result = await response.json();
    setQueryResult(result);
  }

  function onQueryChange(e) {
    document.getElementById("nino_database_query").value = e.target.value;
  }

  React.useEffect(() => {
    aliasesRefresh();
  }, []);

  const aliasesOptions = [];
  for (var alias of aliases) {
    aliasesOptions.push(
      <option {... ((alias.db_alias == "_main") ? { selected: "" } : {})}>{alias.db_alias}</option>
    );
  }

  const resultCols = [];
  if (queryResult.error) {
    resultCols.push(<th>!!! ERROR !!!</th>);
  } else if (!queryResult.cols) {
    resultCols.push(<th></th>);
  } else {
    for (var col of queryResult.cols) {
      resultCols.push(<th>{col.name}<br />{col.type}</th>);
    }
  }

  const resultRows = [];
  if (queryResult.error) {
    resultRows.push(<tr><td>{queryResult.error}</td></tr>);
  } else if (!queryResult.rows) {
    resultRows.push(<tr><td>no result</td></tr>);
  } else {
    for (var row of queryResult.rows) {
      const r = [];
      for (var v of row) {
        r.push(<td>{v}</td>)
      }
      resultRows.push(<tr>{r}</tr>);
    }
  }

  return (
    <div class="row">
      <div class="col-12 col-lg-12">
        <div class="card">
          <div class="card-header">

            <div class="row">

              <div class="col-lg-6 col-12">
                <div class="row">
                  <label class="col-3 col-form-label">DB Alias</label>

                  <div class="col-auto">
                    <select id="nino_database_alias" class="form-select">
                      {aliasesOptions}
                    </select>
                  </div>
                  <div class="col-1">
                    <button type="button" class="btn btn-primary" title="Refresh DB Aliases" onClick={aliasesRefresh}><i class="align-middle" data-feather="refresh-ccw"></i></button>
                  </div>
                </div>
              </div>

              <div class="col-lg-6 col-12">
                <div class="row">
                  <label class="col-3 col-form-label">Query</label>
                  <div class="col-auto">
                    <select class="form-select" onChange={onQueryChange}>
                      <option value="SELECT * FROM information_schema.tables;">all tables</option>
                      <option value="SELECT * FROM information_schema.columns where table_name = 'nino_setting'; ">all columns</option>
                    </select>
                  </div>
                  <div class="col-1">
                    <button type="button" class="btn btn-primary" title="Execute Query" onClick={executeQuery}><i class="align-middle" data-feather="play"></i></button>
                  </div>
                </div>
              </div>

            </div>

          </div>
          <div class="card-body">

            <textarea id="nino_database_query" class="form-control" rows="4" placeholder="Query">select * from information_schema.tables;</textarea>

            <br />
            <div style={{ "overflow-x": "scroll" }}>
              <table class="table table-hover my-0">
                <thead>
                  <tr>
                    {resultCols}
                  </tr>
                </thead>
                <tbody>
                  {resultRows}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}