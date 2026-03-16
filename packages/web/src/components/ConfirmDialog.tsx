import { Description, Dialog, DialogBackdrop, DialogPanel, DialogTitle, Transition, TransitionChild } from "@headlessui/react";
import { Fragment } from "react";

interface ConfirmDialogProps {
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
  title: string;
  description: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
}

export function ConfirmDialog({
  open,
  onClose,
  onConfirm,
  title,
  description,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  danger = false,
}: ConfirmDialogProps) {
  return (
    <Transition show={open} as={Fragment}>
      <Dialog onClose={onClose} className="relative z-50">
        <TransitionChild
          as={Fragment}
          enter="ease-out duration-200"
          enterFrom="opacity-0"
          enterTo="opacity-100"
          leave="ease-in duration-150"
          leaveFrom="opacity-100"
          leaveTo="opacity-0"
        >
          <DialogBackdrop className="fixed inset-0 bg-black/60" />
        </TransitionChild>

        <div className="fixed inset-0 flex items-center justify-center p-4">
          <TransitionChild
            as={Fragment}
            enter="ease-out duration-200"
            enterFrom="opacity-0 scale-95"
            enterTo="opacity-100 scale-100"
            leave="ease-in duration-150"
            leaveFrom="opacity-100 scale-100"
            leaveTo="opacity-0 scale-95"
          >
            <DialogPanel className="w-full max-w-sm rounded-xl bg-bg-secondary border border-border p-6 shadow-xl">
              <DialogTitle className="text-base font-semibold text-text-primary">
                {title}
              </DialogTitle>
              <Description className="mt-2 text-sm text-text-secondary">
                {description}
              </Description>
              <div className="mt-6 flex justify-end gap-3">
                <button
                  onClick={onClose}
                  className="px-4 py-2 text-sm rounded-lg bg-bg-tertiary border border-border text-text-secondary hover:bg-bg-hover hover:text-text-primary transition-colors"
                >
                  {cancelLabel}
                </button>
                <button
                  onClick={() => {
                    onConfirm();
                    onClose();
                  }}
                  className={`px-4 py-2 text-sm rounded-lg border transition-colors ${
                    danger
                      ? "bg-red-500/15 border-red-500/40 text-red-400 hover:bg-red-500/25 hover:text-red-300"
                      : "bg-accent/15 border-accent/40 text-accent hover:bg-accent/25"
                  }`}
                >
                  {confirmLabel}
                </button>
              </div>
            </DialogPanel>
          </TransitionChild>
        </div>
      </Dialog>
    </Transition>
  );
}
