import React from 'react';
import { CheckCircle2, AlertTriangle, X } from 'lucide-react';
import { toast as sonnerToast } from 'sonner';

interface ToastProps {
  message: string;
  type: 'success' | 'error';
  id: string | number;
}

const NotificationToast = ({ message, type, id }: ToastProps) => {
  const isSuccess = type === 'success';

  return (
    <div className={`
      relative flex items-center justify-between min-w-[350px] p-4 rounded-xl 
      backdrop-blur-[40px] border border-white/20 shadow-2xl
      transition-all duration-300 animate-in fade-in slide-in-from-right-5
      ${isSuccess 
        ? 'bg-gradient-to-r from-amber-500/20 to-yellow-600/20 shadow-[0_0_20px_rgba(251,191,36,0.3)]' 
        : 'bg-gradient-to-r from-red-500/20 to-rose-600/20 shadow-[0_0_20px_rgba(244,63,94,0.2)]'}
    `}>
      {/* Golden Glow Effect for Success */}
      {isSuccess && (
        <div className="absolute inset-0 rounded-xl bg-amber-400/5 blur-xl -z-10" />
      )}

      <div className="flex items-center gap-3">
        {isSuccess ? (
          <CheckCircle2 className="w-6 h-6 text-amber-400" />
        ) : (
          <AlertTriangle className="w-6 h-6 text-rose-400" />
        )}
        <p className="text-sm font-medium text-white/90">
          {message}
        </p>
      </div>

      <button 
        onClick={() => sonnerToast.dismiss(id)}
        className="ml-4 p-1 hover:bg-white/10 rounded-full transition-colors text-white/40 hover:text-white"
        title="Close notification"
      >
        <X className="w-4 h-4" />
      </button>
    </div>
  );
};

// Export a helper function to trigger it easily
export const showToast = (message: string, type: 'success' | 'error' = 'success') => {
  sonnerToast.custom((id) => (
    <NotificationToast id={id} message={message} type={type} />
  ), { duration: 4000 });
};