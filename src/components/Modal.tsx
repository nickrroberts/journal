interface ModalButton {
  label: string;
  onClick: () => void;
}

interface ModalProps {
  visible: boolean;
  header: React.ReactNode;
  body: React.ReactNode;
  onClose: () => void;
  secondaryButton?: ModalButton;
  primaryButton?: ModalButton;
}

function Modal({ visible, header, body, onClose, secondaryButton, primaryButton }: ModalProps) {
  if (!visible) return null;
  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg p-6 max-w-lg w-full">
        <h2 className="text-xl font-semibold mb-4 text-black">{header}</h2>
        <div className="mb-4">{body}</div>
        <div className="mt-4 flex justify-end space-x-2">
          {secondaryButton && (
            <button
              onClick={secondaryButton.onClick}
              className="px-4 py-2 bg-gray-200 text-gray-800 rounded hover:bg-gray-300"
            >
              {secondaryButton.label}
            </button>
          )}
          {primaryButton ? (
            <button
              onClick={primaryButton.onClick}
              className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700"
            >
              {primaryButton.label}
            </button>
          ) : (
            <button
              onClick={onClose}
              className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700"
            >
              Close
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

export default Modal;