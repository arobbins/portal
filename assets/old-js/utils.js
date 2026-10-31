function buildClassNameString(settings) {
  const { prefix, attributes, instance } = settings;
  const defaultComponentFields = standardComponentFields();

  return attributes.reduce((prevValue, attr) => {
    const allAttrs = {
      ...defaultComponentFields,
      ...attr,
    };

    if (allAttrs.setsText || allAttrs.copyToInner || allAttrs.relocate) {
      return prevValue;
    }

    if (allAttrs.state) {
      return instance.hasAttribute(allAttrs.name)
        ? prevValue + ` ${prefix}--is-` + allAttrs.name
        : prevValue;
    }

    let value = instance.getAttribute(allAttrs.name);

    if (value === "true") {
      return prevValue + ` ${prefix}--is-` + allAttrs.name;
    }

    if (!value && allAttrs?.fallbackValue) {
      value = allAttrs.fallbackValue;
    }

    if (value) {
      return prevValue + ` ${prefix}--` + value;
    }

    return prevValue;
  }, prefix);
}

function standardComponentFields(instance) {
  return {
    copyToInner: false,
    relocate: false,
    fallbackValue: null,
  };
}

function getAttributeNamesOnly(attributes) {
  return attributes.map((attr) => attr.name);
}

function getDynamicElementType(elementType) {
  return elementType.split("${")[1].split("}")[0];
}

function createWebComponent(settings) {
  const {
    componentName,
    elementType = "",
    attributes,
    onConnect,
    onRender,
    inner,
  } = settings;

  const attributeNamesOnly = getAttributeNamesOnly(attributes);

  if (elementType.includes("${")) {
    let elementTypeAccessKey = getDynamicElementType(elementType);

    let foundAttr = attributes.filter(
      (attr) => attr.name === elementTypeAccessKey,
    );
  }

  class CustomComponent extends HTMLElement {
    static observedAttributes = attributeNamesOnly;

    connectedCallback() {
      if (elementType && !this[elementType]) {
        this[elementType] = document.createElement(elementType);
        this[elementType].append(...this.childNodes);
        this.append(this[elementType]);
      }
      this.render();

      if (onConnect) {
        onConnect(this, this[elementType]);
      }
    }

    attributeChangedCallback(name, oldValue, newValue) {
      if (oldValue !== newValue) {
        this.render();
      }
    }

    render() {
      const target = elementType ? this[elementType] : this;

      if (!target) {
        return;
      }

      const finalClassName = buildClassNameString({
        prefix: componentName,
        attributes: attributes,
        instance: this,
      });

      if (finalClassName) {
        target.className = finalClassName;
      }

      target.disabled = this.hasAttribute("disabled");

      attributes.forEach((attr) => {
        if (attr.setsText) {
          const textValue = this.getAttribute(attr.name);
          if (textValue !== null) {
            target.textContent = textValue;
          }
        }

        if (attr.copyToInner) {
          const value = this.getAttribute(attr.name);

          if (value !== null) {
            target.setAttribute(attr.name, value);
          }
        }

        if (attr.relocate && target !== this) {
          const value = this.getAttribute(attr.name);

          if (value !== null) {
            target.setAttribute(attr.name, value);
            this.removeAttribute(attr.name);
          }
        }
      });

      if (onRender) {
        onRender(this, target);
      }
    }
  }

  customElements.define(componentName, CustomComponent);
}

export { createWebComponent };
