// src/modules/choiceDialog.ts
// Generic modals: N-button choice and one-line text prompt.
// Resolve with the chosen value / entered text, or null on Escape/backdrop.
// Reuses the app's .modal-overlay / .nx-modal styling.

let overlayEl: HTMLDivElement | null = null;
let promptOverlayEl: HTMLDivElement | null = null;

function ensureDom(): HTMLDivElement {
  if (overlayEl) return overlayEl;
  overlayEl = document.createElement('div');
  overlayEl.className = 'modal-overlay hidden';
  overlayEl.setAttribute('aria-hidden', 'true');
  overlayEl.id = 'choice-overlay';

  const modal = document.createElement('div');
  modal.className = 'modal nx-modal';
  modal.id = 'choice-modal';

  const title = document.createElement('h2');
  title.id = 'choice-title';

  const body = document.createElement('pre');
  body.id = 'choice-message';
  body.className = 'nx-mono';
  body.style.whiteSpace = 'pre-wrap';
  body.style.margin = '8px 0';
  body.style.maxHeight = '40vh';
  body.style.overflow = 'auto';
  body.style.textAlign = 'left';

  const actions = document.createElement('div');
  actions.className = 'modal-actions';
  actions.id = 'choice-actions';

  modal.append(title, body, actions);
  modal.addEventListener('click', (e) => e.stopPropagation());
  overlayEl.appendChild(modal);
  document.body.appendChild(overlayEl);
  return overlayEl;
}

export interface ChoiceOption {
  label: string;
  value: string;
  primary?: boolean;
}

export interface ChoiceDialogOpts {
  title: string;
  message: string;
  choices: ChoiceOption[];
}

export function showChoiceDialog({ title, message, choices }: ChoiceDialogOpts): Promise<string | null> {
  const overlay = ensureDom();
  const titleEl = overlay.querySelector('#choice-title') as HTMLElement;
  const msgEl = overlay.querySelector('#choice-message') as HTMLElement;
  const actionsEl = overlay.querySelector('#choice-actions') as HTMLElement;

  titleEl.textContent = title;
  msgEl.textContent = message;
  actionsEl.replaceChildren();

  return new Promise((resolve) => {
    const close = (value: string | null) => {
      overlay.classList.add('hidden');
      overlay.setAttribute('aria-hidden', 'true');
      document.removeEventListener('keydown', onKey, true);
      overlay.onclick = null;
      resolve(value);
    };

    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        close(null);
      }
    };

    let firstBtn: HTMLButtonElement | null = null;
    for (const choice of choices) {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.textContent = choice.label;
      btn.onclick = () => close(choice.value);
      actionsEl.appendChild(btn);
      if (choice.primary && !firstBtn) firstBtn = btn;
    }
    if (!firstBtn) firstBtn = actionsEl.querySelector('button');

    overlay.onclick = (e) => {
      if (e.target === overlay) close(null);
    };
    document.addEventListener('keydown', onKey, true);

    overlay.classList.remove('hidden');
    overlay.setAttribute('aria-hidden', 'false');
    setTimeout(() => firstBtn?.focus(), 0);
  });
}

function ensurePromptDom(): HTMLDivElement {
  if (promptOverlayEl) return promptOverlayEl;
  promptOverlayEl = document.createElement('div');
  promptOverlayEl.className = 'modal-overlay hidden';
  promptOverlayEl.setAttribute('aria-hidden', 'true');
  promptOverlayEl.id = 'prompt-overlay';

  const modal = document.createElement('div');
  modal.className = 'modal nx-modal';

  const title = document.createElement('h2');
  title.id = 'prompt-title';

  const input = document.createElement('input');
  input.type = 'text';
  input.id = 'prompt-input';

  const actions = document.createElement('div');
  actions.className = 'modal-actions';

  const ok = document.createElement('button');
  ok.type = 'button';
  ok.id = 'prompt-ok';
  ok.textContent = 'OK';
  const cancel = document.createElement('button');
  cancel.type = 'button';
  cancel.id = 'prompt-cancel';
  cancel.textContent = 'CANCEL';
  actions.append(ok, cancel);

  modal.append(title, input, actions);
  modal.addEventListener('click', (e) => e.stopPropagation());
  promptOverlayEl.appendChild(modal);
  document.body.appendChild(promptOverlayEl);
  return promptOverlayEl;
}

export interface PromptDialogOpts {
  title: string;
  initial?: string;
}

export function showPromptDialog({ title, initial = '' }: PromptDialogOpts): Promise<string | null> {
  const overlay = ensurePromptDom();
  const titleEl = overlay.querySelector('#prompt-title') as HTMLElement;
  const input = overlay.querySelector('#prompt-input') as HTMLInputElement;
  const okBtn = overlay.querySelector('#prompt-ok') as HTMLButtonElement;
  const cancelBtn = overlay.querySelector('#prompt-cancel') as HTMLButtonElement;

  titleEl.textContent = title;
  input.value = initial;

  return new Promise((resolve) => {
    const close = (value: string | null) => {
      overlay.classList.add('hidden');
      overlay.setAttribute('aria-hidden', 'true');
      input.onkeydown = null;
      okBtn.onclick = null;
      cancelBtn.onclick = null;
      overlay.onclick = null;
      resolve(value);
    };

    okBtn.onclick = () => close(input.value);
    cancelBtn.onclick = () => close(null);
    overlay.onclick = (e) => {
      if (e.target === overlay) close(null);
    };
    input.onkeydown = (ev: KeyboardEvent) => {
      ev.stopPropagation();
      if (ev.key === 'Escape') { ev.preventDefault(); close(null); }
      if (ev.key === 'Enter') { ev.preventDefault(); close(input.value); }
    };

    overlay.classList.remove('hidden');
    overlay.setAttribute('aria-hidden', 'false');
    setTimeout(() => { input.focus(); input.select(); }, 0);
  });
}
