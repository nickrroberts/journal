interface ModalProps {
    visible: boolean;
    header: React.ReactNode;
    body: React.ReactNode;
    onClose: () => void;
  }
  
  function Modal({ visible, header, body, onClose }: ModalProps) {
    if (!visible) return null;
    return (
      <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
        <div className="bg-white rounded-lg p-6 max-w-lg w-full">
          <h2 className="text-xl font-semibold mb-4">{header}</h2>
          <div className="mb-4">{body}</div>
          <button
            onClick={onClose}
            className="mt-2 px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700"
          >
            Close
          </button>
        </div>
      </div>
    );
  }

  export default Modal;