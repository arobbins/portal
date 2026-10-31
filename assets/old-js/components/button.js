import { createWebComponent } from "../utils.js";

function createButtonComponent(elementType) {
  createWebComponent({
    componentName: "a-button",
    elementType: "button",
    attributes: [
      {
        name: "variant",
        fallbackValue: "primary",
      },
      {
        name: "size",
        fallbackValue: "medium",
      },
      {
        name: "busy",
      },
      {
        name: "disabled",
      },
      {
        name: "command",
        copyToInner: true,
      },
      {
        name: "commandfor",
        copyToInner: true,
      },
      {
        name: "label",
        setsText: true,
      },
    ],
  });
}

export default createButtonComponent;
