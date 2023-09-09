import React from 'react';
import EditorRequests from './EditorRequests';
import EditorDB from './EditorDB';
import EditorDynamics from './EditorDynamics';

function instantiateObjectID(objectID) {
  var component;
  if (objectID.startsWith("db:")) {
    component = <EditorDB objectID={objectID} />;
  } else if (objectID.startsWith("path:")) {
    component = <EditorRequests objectID={objectID} />;
  } else if (objectID.startsWith("dynamic:")) {
    component = <EditorDynamics objectID={objectID} />;
  }
  return component;
}

function EditorTabs({ IDEContext }) {
  function activateTab(objectID) {
    IDEContext.tabSelected = objectID;
    IDEContext.tabSetSelected(objectID);
  }

  function closeTab(objectID) {
    var tabs = IDEContext.tabs;
    for (var t = 0; t < tabs.length; t++) {
      var tab = tabs[t];
      if (tab.objectID === objectID) {
        for (var s = t; s < tabs.length - 1; s++) {
          //shift
          tabs[s] = tabs[s + 1];
        }
        tabs.pop()
        if (t >= tabs.length) {
          t = tabs.length - 1;
        }
        if (t >= 0) {
          tab = tabs[t]
          activateTab(tab.objectID);
        } else {
          activateTab("");
        }
        break;
      }
    }
  }

  function mapTabs(tab) {
    return <div key={tab.objectID} className={"nino-ide-editor-tab" + (tab.objectID === IDEContext.tabSelected ? " tab-active" : "")}>
      &nbsp;&nbsp;
      <div className='tab-link tab-text' onClick={() => activateTab(tab.objectID)}>{tab.objectID}</div>
      &nbsp;&nbsp;
      <div className='tab-link tab-close' onClick={() => closeTab(tab.objectID)}>x</div>
      &nbsp;&nbsp;
    </div>
  }
  return (
    <div className='nino-ide-editor-tabs'>
      {IDEContext.tabs.map(mapTabs)}
    </div>
  );
}

function EditorToggable({ IDEContext }) {
  function mapTabsToggle(tab) {
    return (
      <div key={tab.objectID} style={{
        display: tab.objectID === IDEContext.tabSelected ? "block" : "none",
        height: "calc(100% - 11px)", padding: "5px"
      }}>
        {tab.component}
      </div>
    );
  }
  return (
    <div className='nino-ide-editor-container'>
      {IDEContext.tabs.map(mapTabsToggle)}
    </div>
  );
}

export default function Editors({ IDEContext }) {
  const [selected, setSelected] = React.useState("");
  IDEContext.tabSelected = selected;
  IDEContext.tabSetSelected = setSelected;
  IDEContext.addTab = function (objectID) {
    var component = instantiateObjectID(objectID);

    var tab = {
      objectID: objectID,
      component: component,
    }
    IDEContext.tabs.push(tab);
    IDEContext.tabSelected = objectID;
    IDEContext.tabSetSelected(objectID);
  }

  return (
    <div id='nino-ide-editors'>
      <div className='nino-ide-editor-tabs-background'>
      </div>
      <EditorTabs IDEContext={IDEContext} />
      <EditorToggable IDEContext={IDEContext} />
    </div>
  );
};