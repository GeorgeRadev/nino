import log from "_log";
import nino from "_nino";

export default async function portal_rest(request) {
  await nino.assertRole(request, 'admin');

  log(JSON.stringify(request) + "\n");
  var op = request.parameters['op'];
  if (!op || !op[0]) {
    return null;
  }

  switch (op[0]) {
    case '/requests/get':
      return await nino.ninoRequestsGet();
    case '/requests/detail':
      debugger;
      {
        const name = request.parameters['name'];
        if (!name || !name[0]) {
          return { error: "no query parameter 'name' provided" };
        }
        return await nino.ninoRequestsDetail(name[0]);
      }
    case '/responses/get':
      return await nino.ninoResponsesGet();
    case '/responses/detail':
      {
        const name = request.parameters['name'];
        if (!name || !name[0]) {
          return { error: "no query parameter 'name' provided" };
        }
        return await nino.ninoResponsesDetail(name[0]);
      }
    case '/users/get':
      return await nino.ninoUsersRolesGet();
    case '/portlets/get':
      return await nino.ninoPortletsGet();
    case '/settings/get':
      return await nino.ninoSettingsGet();
    case '/databases/get':
      return await nino.ninoDatabasesGet();
    case '/databases/query':
      {
        const alias = request.parameters['alias'];
        if (!alias || !alias[0]) {
          return { error: "no query parameter 'alias' provided" };
        }
        const query = request.parameters['query'];
        if (!query || !query[0]) {
          return { error: "no query parameter 'query' provided" };
        }
        return await nino.ninoDatabaseQuery(alias[0], query[0]);
      }
    case '/logs/get':
      {
        const limit = request.parameters['limit'];
        if (limit && limit[0]) {
          limit = +limit;
        }
        if (limit != limit) {
          limit = 50;
        }
        return await nino.ninoLogsGet(limit);
      }
  }
  throw new Error("unrecognized portal operation: " + op[0]);
}