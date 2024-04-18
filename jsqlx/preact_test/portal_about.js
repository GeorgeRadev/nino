import React from 'react';

export default function portal_about() {
    // State to store count value
    const [count, setCount] = React.useState(0);

    // Function to increment count by 1
    const incrementCount = () => {
        // Update state with incremented value
        setCount(count + 1);
    };
    return /*#__PURE__*/React.createElement("div", {
        className: "app"
    }, /*#__PURE__*/React.createElement("button", {
        onClick: incrementCount
    }, "Click Here"), count);
}