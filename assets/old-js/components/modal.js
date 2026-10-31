import { createWebComponent } from "../utils.js";

function createModalComponent() {
  createWebComponent({
    componentName: "a-modal",
    elementType: "dialog",
    attributes: [
      {
        name: "open",
      },
      {
        name: "id",
        relocate: true,
      },
      {
        name: "closedby",
        copyToInner: true,
      },
      {
        name: "heading",
        elementType: "h2",
        copyToText: true,
      },
    ],
    onConnect: (host, dialog) => {
      dialog.addEventListener("close", () => {
        host.dispatchEvent(new CustomEvent("close", { bubbles: true }));
      });

      if (!dialog.querySelector(".a-modal__close-icon")) {
        dialog.insertAdjacentHTML(
          "beforeend",
          `<button command="close" commandfor="${dialog.id}" aria-label="Close" class="a-modal__close-icon">✕</button>`,
        );
      }
    },
    onRender: (host, dialog) => {
      const shouldBeOpen = host.hasAttribute("open");

      if (shouldBeOpen && !dialog.open) {
        dialog.showModal();
      }

      if (!shouldBeOpen && dialog.open) {
        dialog.close();
      }
    },
  });
}

export default createModalComponent;
