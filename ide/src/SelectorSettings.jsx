import React from 'react';
import Dialog from './Dialog';
import NinoREST from './NinoRest';

const prefix = "setting:";

async function optionsReload(setOptions) {
    try {
        setOptions(await NinoREST.settingsGet());
    } catch (error) {
        setOptions({ "error": "cannot load settings: " + error.message });
    }
}

function OptionsRender({ options }) {
    const listItems = Object.entries(options).map(([key, value]) => <option key={key} value={key}>{key + " = " + value}</option>);
    return (<>{listItems}</>);
}

export default function SelectorDB({ IDEContext }) {
    const [dialogVisible, setDialogVisible] = React.useState(false);
    const [options, setOptions] = React.useState({});
    const [selection, setSelection] = React.useState("");
    const [newName, setNewName] = React.useState("");
    const inputRef = React.createRef();

    async function optionsRefresh() {
        await optionsReload(setOptions);
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
        if (dialogVisible && inputRef.current) {
            inputRef.current.focus();
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
            <div className='nino-ide-selector-title'>SETTINGS</div>
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
                <input type="text" className="selector-field" ref={inputRef} value={newName} onInput={e => setNewName(e.target.value)} maxLength="1024" />
            </Dialog>
        </div>
    );
}