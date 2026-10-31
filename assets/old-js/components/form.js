import { createWebComponent } from "../utils.js";

function createFormComponent() {
  createWebComponent({
    componentName: "a-form",
    elementType: "form",
    attributes: [],
  });

  createWebComponent({
    componentName: "a-form-group",
    elementType: "fieldset",
    attributes: [],
  });

  createWebComponent({
    componentName: "a-form-group-col",
    attributes: [
      {
        name: "size",
      },
    ],
  });

  createWebComponent({
    componentName: "a-form-label",
    elementType: "label",
    attributes: [
      {
        name: "for",
        copyToInner: true,
      },
    ],
  });

  createWebComponent({
    componentName: "a-form-input",
    elementType: "input",
    attributes: [
      {
        name: "name",
        copyToInner: true,
      },
      {
        name: "placeholder",
        copyToInner: true,
      },
      {
        name: "type",
        copyToInner: true,
      },
      {
        name: "id",
        relocate: true,
      },
      {
        name: "required",
        copyToInner: true,
      },
    ],
    onConnect: (instance, htmlEl) => {
      let originalVal = htmlEl.value;

      htmlEl.addEventListener("focus", (event) => {
        htmlEl.classList.add("touched");
      });

      htmlEl.addEventListener("input", (event) => {
        if (htmlEl.value !== originalVal) {
          htmlEl.classList.add("changed");
        } else {
          htmlEl.classList.remove("changed");
        }
      });
    },
  });
}

export default createFormComponent;
