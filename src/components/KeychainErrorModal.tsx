import Modal from './Modal';

interface KeychainErrorModalProps {
  visible: boolean;
  onClose: () => void;
  onRetry: () => void;
  errorMessage: string;
}

export default function KeychainErrorModal({ 
  visible, 
  onClose, 
  onRetry,
  errorMessage 
}: KeychainErrorModalProps) {
  return (
    <Modal
      visible={visible}
      header="Keychain Access Required"
      body={
        <div className="space-y-4">
          <p className="text-black">
            Your journal is encrypted for privacy. To continue, we need permission to store a secure key in your macOS Keychain.
          </p>
          <p className="text-black">
            {errorMessage}
          </p>
        </div>
      }
      onClose={onClose}
      primaryButton={{
        label: 'Retry',
        onClick: onRetry
      }}
      secondaryButton={{
        label: 'Close',
        onClick: onClose
      }}
    />
  );
} 