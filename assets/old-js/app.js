import createButtonComponent from "./components/button.js";
import createFormComponent from "./components/form.js";
import createModalComponent from "./components/modal.js";

function createAllWebComponents() {
  createButtonComponent();
  createFormComponent();
  createModalComponent();
}

function main() {
  createAllWebComponents();
}

main();
