import log from "_log";
import nino from "_nino";

export default async function portal_rest(request) {
  debugger;
  await nino.assertRole(request, 'admin');

  log(JSON.stringify(request));
  var op = request.parameters['op'];
  if (!op || !op[0]) {
    return null;
  }

  switch (op[0]) {
    case '/requests/get':
      return await nino.ninoRequestsGet();

  }
  return null;
}