import React from 'react';
import Dialog from './Dialog';


function SelectionOption() {
    var arr = ['1', '2', '3'];
    const listItems = arr.map((e) => <option value={e}>{e}</option>);
    return (<>{listItems}</>);
}

export default function SelectorDB({ IDEContext }) {
    const [visible, setVisible] = React.useState(false);
    const [selection, setSelection] = React.useState("");
    const [newName, setNewName] = React.useState("");
    const inputReference = React.useRef(null);

    React.useEffect(() => {
        if (visible && inputReference.current) {
            inputReference.current.focus();
        }
    }, [visible]);

    function onOk() {
        if (newName) {
            IDEContext.addTab("setting:" + newName);
        }
    }
    function onClose() {
        setVisible(false);

    }
    function selectionNew() {
        setNewName("");
        setVisible(true);
    }
    function selectionEdit() {
        if (selection) {
            IDEContext.addTab("setting:" + selection);
        }
    }
    function selectionClick(event) {
        // console.log("click on: " + event.target.value + " " + event.detail + " times");
        if (event.detail === 1) {
            setSelection(String(event.target.value));
        } else if (event.detail === 2) {
            IDEContext.addTab("setting:" + event.target.value);
        }
    }
    return (
        <div>
            <div className='nino-ide-selector-title'>SETTINGS</div>
            <br />
            <button onClick={selectionNew}>New</button>&nbsp;&nbsp;&nbsp;
            <button onClick={selectionEdit}>Edit</button>
            <br />
            filter:
            <br />
            <input type="text" className="selector_field" name="filter settings" maxLength="1024" />
            <br />
            Settings:
            <br />
            <select className="selector_field" name="cars" size="20" onClick={selectionClick}>
                <SelectionOption />
            </select>
            <br />

            <Dialog visible={visible} onOk={() => onOk()} onClose={() => onClose()} >
                Setting name:&nbsp;&nbsp;&nbsp;
                <input type="text" className="selector_field" ref={inputReference} value={newName} onInput={e => setNewName(e.target.value)} maxLength="1024" />
            </Dialog>
        </div>
    );
}