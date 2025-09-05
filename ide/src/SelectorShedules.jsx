import React from 'react';
import Dialog from './Dialog';

const prefix = "schedule:"
function optionsReload(setOptions) {
    //fetch content
    setOptions(['1', '2', '3']);
}

function OptionsRender({ options }) {
    const listItems = options.map((e) => <option key={e} value={e}>{e}</option>);
    return (<>{listItems}</>);
}

export default function SelectorDB({ IDEContext }) {
    const [dialogVisible, setDialogVisible] = React.useState(false);
    const [options, setOptions] = React.useState([]);
    const [selection, setSelection] = React.useState("");
    const [newName, setNewName] = React.useState("");
    const inputReference = React.useRef(null);

    function optionsRefresh() {
        optionsReload(setOptions);
    }
    React.useEffect(() => {
        optionsRefresh();
    }, []);

    function dialogOpen() {
        setNewName("");
        setDialogVisible(true);
    }
    // select dialog field
    React.useEffect(() => {
        if (dialogVisible && inputReference.current) {
            inputReference.current.focus();
        }
    }, [dialogVisible]);
    function dialogOnOk() {
        if (newName) {
            IDEContext.addTab(prefix + newName);
        }
    }
    function dialogOnClose() {
        setDialogVisible(false);
    }
    function optionsEdit() {
        if (selection) {
            IDEContext.addTab(prefix + selection);
        } else {
            alert("Selection needed to edit it");
        }
    }
    function optionsClick(event) {
        console.log("click on: " + event.target.value + " " + event.detail + " times");
        if (event.detail === 1) {
            setSelection(event.target.value);
        } else if (event.detail === 2) {
            IDEContext.addTab(prefix + event.target.value);
        }
    }
    return (
        <div>
            <div className='nino-ide-selector-title'>SCHEDULES</div>
            <br />
            <button onClick={dialogOpen}>New</button>&nbsp;&nbsp;&nbsp;
            <button onClick={optionsEdit}>Edit</button>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
            <button onClick={optionsRefresh}>Refresh</button>
            <br />
            filter:
            <br />
            <input type="text" className="selector-field" name="filter settings" maxLength="1024" />
            <br />
            Settings:
            <br />
            <select className="selector-field" name="cars" size="20" onClick={optionsClick}>
                <OptionsRender options={options} />
            </select>
            <br />

            <Dialog visible={dialogVisible} onOk={dialogOnOk} onClose={dialogOnClose} >
                Setting name:&nbsp;&nbsp;&nbsp;
                <input type="text" className="selector-field" ref={inputReference} value={newName} onInput={e => setNewName(e.target.value)} maxLength="1024" />
            </Dialog>
        </div>
    );
}