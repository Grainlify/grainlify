import React from 'react'
import { Toaster } from 'sonner'
import { useTheme } from '../contexts/ThemeContext'

const Toast = () => {
  const { theme } = useTheme();
  return (
    <Toaster
      richColors={false}
      position="top-right"
      closeButton={true}
      duration={3000}
      visibleToasts={1}
      expand={false}
      toastOptions={{
        unstyled: true,
        className: `${theme === 'dark'
          ? 'bg-[#2d2820]/80 text-[#f5f5f5] border-[#c9983a]/30'
          : 'bg-white/90 text-[#2d2820] border-[#c9983a]/30'
        } backdrop-blur-[40px] w-[340px] flex flex-row text-md py-3 px-4 rounded-[12px] border shadow-[0_8px_24px_rgba(0,0,0,0.15)]`,
        classNames: {
          closeButton: "order-last ml-auto cursor-pointer",
          icon: "mr-1 mt-0.5",
          description: "mt-0.5 text-sm",
          success: 'border border-[#c9983a]/60 shadow-[0_4px_18px_rgba(201,152,58,0.35)]',
          error: 'border border-[#ff6b6b]/60 shadow-[0_4px_18px_rgba(255,107,107,0.35)]'
        }
      }}
    />
  )
}

export default Toast
