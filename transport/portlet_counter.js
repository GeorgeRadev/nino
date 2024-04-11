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
        <div class="x_panel">
            <div class="x_title">
                <h2>Counter</h2>
                <div class="clearfix"></div>
            </div>
            <div class="x_content">
                <br />
                <form class="form-horizontal form-label-left">
                    <div class="form-group row ">
                        <label class="control-label col-md-3 col-sm-3 ">react hook button</label>
                        <div class="col-md-9 col-sm-9 ">
                            <button type="button" class="btn btn-primary" onClick={incrementCount}>Click to increment</button>
                        </div>
                    </div>
                    <div class="form-group row">
                        <label class="control-label col-md-3 col-sm-3 ">Incremented value </label>
                        <div class="col-md-9 col-sm-9 ">
                            <span>{count}</span>
                        </div>
                    </div>
                </form>
            </div>
        </div>
    );
}