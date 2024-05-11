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
        <div class="row">
            <div class="col-12 col-lg-12">
                <div class="card">
                    <div class="card-header">
                        <h5 class="card-title">Increment Counter Portlet</h5>
                    </div>
                    <div class="card-body">
                        Click <button type="button" class="btn btn-primary" onClick={incrementCount}>the button</button> to increment the value: <strong>{count}</strong>
                    </div>
                </div>
            </div>
        </div>
    );
}