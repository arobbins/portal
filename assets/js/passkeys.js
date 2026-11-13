const GENERIC_ERROR = "We couldn't use that passkey. Please try again.";
const CANCELLED = "The passkey prompt was closed before it finished.";
const LOGIN_ERROR_ID = "passkey-error";
const REGISTER_ERROR_ID = "add-passkey-error";

const base64url = {
  encode(buffer) {
    const bytes = new Uint8Array(buffer);
    let binary = "";
    for (const byte of bytes) binary += String.fromCharCode(byte);
    return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
  },
  decode(text) {
    const base64 = text.replace(/-/g, "+").replace(/_/g, "/");
    const binary = atob(base64.padEnd(base64.length + ((4 - (base64.length % 4)) % 4), "="));
    return Uint8Array.from(binary, (char) => char.charCodeAt(0)).buffer;
  },
};

function creationOptions(json) {
  if (PublicKeyCredential.parseCreationOptionsFromJSON) {
    return PublicKeyCredential.parseCreationOptionsFromJSON(json);
  }
  return {
    ...json,
    challenge: base64url.decode(json.challenge),
    user: { ...json.user, id: base64url.decode(json.user.id) },
    excludeCredentials: (json.excludeCredentials ?? []).map((credential) => ({
      ...credential,
      id: base64url.decode(credential.id),
    })),
  };
}

function requestOptions(json) {
  if (PublicKeyCredential.parseRequestOptionsFromJSON) {
    return PublicKeyCredential.parseRequestOptionsFromJSON(json);
  }
  return {
    ...json,
    challenge: base64url.decode(json.challenge),
    allowCredentials: (json.allowCredentials ?? []).map((credential) => ({
      ...credential,
      id: base64url.decode(credential.id),
    })),
  };
}

function credentialJSON(credential) {
  if (typeof credential.toJSON === "function") return credential.toJSON();
  const response = credential.response;
  const json = {
    id: credential.id,
    rawId: base64url.encode(credential.rawId),
    type: credential.type,
    clientExtensionResults: credential.getClientExtensionResults(),
    response: { clientDataJSON: base64url.encode(response.clientDataJSON) },
  };
  if (response.attestationObject) {
    json.response.attestationObject = base64url.encode(response.attestationObject);
    json.response.transports = response.getTransports?.() ?? [];
  } else {
    json.response.authenticatorData = base64url.encode(response.authenticatorData);
    json.response.signature = base64url.encode(response.signature);
    if (response.userHandle) json.response.userHandle = base64url.encode(response.userHandle);
  }
  return json;
}

async function post(url, body) {
  try {
    const response = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      credentials: "same-origin",
      body: JSON.stringify(body),
    });
    const data = await response.json().catch(() => ({ error: GENERIC_ERROR }));
    return { ok: response.ok, data };
  } catch {
    return { ok: false, data: { error: GENERIC_ERROR } };
  }
}

function showError(id, message) {
  const slot = document.getElementById(id);
  if (!slot) return;
  slot.querySelector(".a-notice").textContent = message;
  slot.classList.remove("a-form-error--is-hidden");
}

function hideError(id) {
  document.getElementById(id)?.classList.add("a-form-error--is-hidden");
}

function promptError(error, origin) {
  if (error?.name === "NotAllowedError") return CANCELLED;
  if (error?.name === "SecurityError" && origin) {
    return `Passkeys only work at ${origin}. Open that address and try again.`;
  }
  return GENERIC_ERROR;
}

async function signalAccepted(details) {
  if (!details?.rpId) return;
  try {
    await PublicKeyCredential.signalAllAcceptedCredentials?.({
      rpId: details.rpId,
      userId: details.userId,
      allAcceptedCredentialIds: details.allAcceptedCredentialIds,
    });
    await PublicKeyCredential.signalCurrentUserDetails?.({
      rpId: details.rpId,
      userId: details.userId,
      name: details.name,
      displayName: details.displayName,
    });
  } catch {}
}

async function register(form) {
  const button = form.querySelector('button[type="submit"]');
  const password = form.querySelector('input[name="current_password"]');
  hideError(REGISTER_ERROR_ID);
  password?.removeAttribute("aria-invalid");
  button.disabled = true;

  try {
    const start = await post(
      "/passkeys/register/start",
      password ? { current_password: password.value } : {},
    );
    if (!start.ok) {
      if (password && start.data.field === "current_password") {
        password.setAttribute("aria-invalid", "true");
      }
      showError(REGISTER_ERROR_ID, start.data.error ?? GENERIC_ERROR);
      return;
    }

    let credential;
    try {
      credential = await navigator.credentials.create({
        publicKey: creationOptions(start.data.options.publicKey),
      });
    } catch (error) {
      showError(REGISTER_ERROR_ID, promptError(error, start.data.origin));
      return;
    }

    const finish = await post("/passkeys/register/finish", credentialJSON(credential));
    if (!finish.ok) {
      showError(REGISTER_ERROR_ID, finish.data.error ?? GENERIC_ERROR);
      return;
    }
    if (form.dataset.passkeyDone) {
      location.assign(form.dataset.passkeyDone);
    } else {
      location.reload();
    }
  } finally {
    button.disabled = false;
  }
}

let pendingLogin = null;

async function finishLogin(credential) {
  const finish = await post("/passkeys/login/finish", credentialJSON(credential));
  if (finish.ok) {
    await signalAccepted(finish.data);
    location.assign(finish.data.redirect);
    return;
  }
  if (finish.data.unknownCredential) {
    try {
      await PublicKeyCredential.signalUnknownCredential?.({
        rpId: location.hostname,
        credentialId: finish.data.unknownCredential,
      });
    } catch {}
  }
  showError(LOGIN_ERROR_ID, finish.data.error ?? GENERIC_ERROR);
  startAutofill();
}

async function startAutofill() {
  if (!(await PublicKeyCredential.isConditionalMediationAvailable?.())) return;
  if (!document.querySelector('input[autocomplete~="webauthn"]')) return;

  pendingLogin?.abort();
  const controller = new AbortController();
  pendingLogin = controller;
  const start = await post("/passkeys/login/start", { conditional: true });
  if (!start.ok || controller.signal.aborted) return;

  try {
    const credential = await navigator.credentials.get({
      publicKey: requestOptions(start.data.options.publicKey),
      mediation: "conditional",
      signal: controller.signal,
    });
    await finishLogin(credential);
  } catch {}
}

async function signInWithButton(button) {
  pendingLogin?.abort();
  pendingLogin = null;
  hideError(LOGIN_ERROR_ID);
  button.disabled = true;

  try {
    const start = await post("/passkeys/login/start", { conditional: false });
    if (!start.ok) {
      showError(LOGIN_ERROR_ID, start.data.error ?? GENERIC_ERROR);
      return;
    }
    let credential;
    try {
      credential = await navigator.credentials.get({
        publicKey: requestOptions(start.data.options.publicKey),
      });
    } catch (error) {
      showError(LOGIN_ERROR_ID, promptError(error, start.data.origin));
      startAutofill();
      return;
    }
    await finishLogin(credential);
  } finally {
    button.disabled = false;
  }
}

function setUp() {
  if (!window.PublicKeyCredential) {
    for (const element of document.querySelectorAll("[data-passkey-login], [data-passkey-register]")) {
      element.closest(".passkey-sign-in")?.setAttribute("hidden", "");
    }
    for (const element of document.querySelectorAll("[data-passkey-only]")) {
      element.setAttribute("hidden", "");
    }
    return;
  }

  const signals = document.querySelector("[data-passkey-signals]");
  if (signals) signalAccepted(JSON.parse(signals.dataset.passkeySignals));

  startAutofill();
}

document.addEventListener("submit", (event) => {
  const form = event.target.closest("[data-passkey-register]");
  if (!form) return;
  event.preventDefault();
  register(form);
});

document.addEventListener("click", (event) => {
  const button = event.target.closest("[data-passkey-login]");
  if (!button) return;
  event.preventDefault();
  signInWithButton(button);
});

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", setUp);
} else {
  setUp();
}
