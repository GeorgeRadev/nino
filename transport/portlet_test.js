import React from 'react';

export default function portlet_test() {
    // State to store count value
    const [count, setCount] = React.useState(0);

    // Function to increment count by 1
    const incrementCount = () => {
        // Update state with incremented value
        setCount(count + 1);
    };
    return (
        <div>
            <button onClick={incrementCount}>Click to increment</button>
            <span>{count}</span>
        </div>
    );
}