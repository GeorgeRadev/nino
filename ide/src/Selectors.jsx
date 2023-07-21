import React from 'react';
import SelectorDB from './SelectorDB';
import SelectorRequests from './SelectorRequests';

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

        <SelectorToggable name="databases" IDEContext={IDEContext}>
          <SelectorDB IDEContext={IDEContext} />
        </SelectorToggable>
      </div>
    </div>
  );
};