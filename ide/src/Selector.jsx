import React from 'react';
import SelectorRequests from './SelectorRequests';
import SelectorStatics from './SelectorStatics';
import SelectorDynamics from './SelectorDynamics';
import SelectorDatabases from './SelectorDatabases';
import SelectorRoles from './SelectorRoles';
import SelectorUsers from './SelectorUsers';
import SelectorShedules from './SelectorShedules';
import SelectorTransports from './SelectorTransports';
import SelectorSettings from './SelectorSettings';

function SelectorToggable({ name, IDEContext, children }) {
  const selectedAction = IDEContext.selectedAction;
  return (
    <div style={{ display: selectedAction === name ? "block" : "none" }}>
      {children}
    </div>
  );
}

export default function Selectors({ IDEContext }) {
  const [repaint, repaintViewers] = React.useState();
  IDEContext.repaintViewers = repaintViewers
  IDEContext.repaint = repaint

  return (
    <div id='nino-ide-selectors'>
      <div style={{ padding: "6px" }}>
        <SelectorToggable name="requests" IDEContext={IDEContext}>
          <SelectorRequests IDEContext={IDEContext} />
        </SelectorToggable>

        <SelectorToggable name="statics" IDEContext={IDEContext}>
          <SelectorStatics IDEContext={IDEContext} />
        </SelectorToggable>

        <SelectorToggable name="dynamics" IDEContext={IDEContext}>
          <SelectorDynamics IDEContext={IDEContext} />
        </SelectorToggable>

        <SelectorToggable name="databases" IDEContext={IDEContext}>
          <SelectorDatabases IDEContext={IDEContext} />
        </SelectorToggable>

        <SelectorToggable name="roles" IDEContext={IDEContext}>
          <SelectorRoles IDEContext={IDEContext} />
        </SelectorToggable>

        <SelectorToggable name="users" IDEContext={IDEContext}>
          <SelectorUsers IDEContext={IDEContext} />
        </SelectorToggable>

        <SelectorToggable name="schedules" IDEContext={IDEContext}>
          <SelectorShedules IDEContext={IDEContext} />
        </SelectorToggable>

        <SelectorToggable name="transports" IDEContext={IDEContext}>
          <SelectorTransports IDEContext={IDEContext} />
        </SelectorToggable>

        <SelectorToggable name="settings" IDEContext={IDEContext}>
          <SelectorSettings IDEContext={IDEContext} />
        </SelectorToggable>
      </div>
    </div>
  );
};