import React from 'react';

export default function portlet_menu() {
    //get current portlet selection from the URL
    var currentPortletName = window.location.toString().split('=')[1] || "";
    if (currentPortletName.length > 0 && currentPortletName.charAt(0) === '/') {
        currentPortletName = currentPortletName.substring(1);
    }
    // get first portlet as home when not found
    const portlet_menu = window.portlet_menu;
    var currentPortlet = portlet_menu[currentPortletName];
    if (!currentPortlet) {
        for (var path in window.portlet_menu) {
            currentPortlet = portlet_menu[path];
            break;
        }
    }
    const menuItems = [];

    for (var path in window.portlet_menu) {
        const value = window.portlet_menu[path];
        if (path.startsWith("separator")) {
            // menu separator {value.level}
            menuItems.push(<li class="sidebar-header">{value.name}</li>);
        } else {
            // link to portlet
            var path_last_token = path.split("/").pop();

            menuItems.push(<li class={(currentPortlet.path === path) ? "sidebar-item active" : "sidebar-item"}>
                <a class="sidebar-link" href={"/portal?portlet=/" + path}>
                    <i class="align-middle" data-feather={value.icon}></i>
                    <span class="align-middle">{path_last_token}</span>
                </a>
            </li>);
        }
    }

    return (<ul class="sidebar-nav">{menuItems}</ul>);
}