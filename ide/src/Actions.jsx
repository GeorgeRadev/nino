import React from 'react';

export const ActionDetails = [
  { name: "separator1" },
  { name: 'requests', image: 'rebase', alt: "request paths" },
  { name: 'statics', image: 'upload_file', alt: "statics" },
  { name: 'dynamics', image: 'file_present', alt: "dynamics" },
  { name: 'users', image: 'school', alt: "Roles and Users" },
  { name: 'databases', image: 'database', alt: "database" },
  { name: 'schedules', image: 'update', alt: "schedule" },
  { name: "separator2" },
  { name: 'settings', image: 'settings', alt: "settings" },
]


function Separator() {
  return <li >&nbsp;</li>;
}

function Action({ name, image, alt, IDEContext }) {
  const selectedAction = IDEContext.selectedAction;
  function setSelection(selectedAction) {
    IDEContext.setSelectedAction(selectedAction);
    IDEContext.selectedAction = selectedAction;
    IDEContext.repaintViewers(selectedAction);
  }
  return (
    <li key={name} className={selectedAction === name ? "active" : ""} >
      <a href={"#" + name} alt={alt} title={alt} onClick={() => setSelection(name)}>
        <span className="material-symbols-outlined">{image}</span>
      </a>
    </li>
  );
}

export default function Actions({ IDEContext }) {
  const [selectedAction, setSelectedAction] = React.useState('requests');
  IDEContext.setSelectedAction = setSelectedAction
  IDEContext.selectedAction = selectedAction

  function mapActions(action) {
    if (action.name.startsWith('separator')) {
      return <Separator key={action.name} />;
    } else {
      return <Action key={action.name} name={action.name} image={action.image} alt={action.alt} IDEContext={IDEContext} />
    }
  }

  return (
    <div id='nino-ide-actions'>
      <ul>
        <li >
          <img src='sf.svg' width={42} alt='Nino IDE' />
        </li>
        {ActionDetails.map(mapActions)}
      </ul>
    </div>
  );
};