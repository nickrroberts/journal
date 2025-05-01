/** @type {import('tailwindcss').Config} */
module.exports = {
    content: [
      './index.html',
      './src/**/*.{js,ts,jsx,tsx,html}',
    ],
    darkMode: 'class', // or 'media' for OS-level preference
    theme: {
      extend: {
        colors: {
          primary: '#1E40AF',    // Indigo-800
          secondary: '#F59E0B',  // Amber-500
        },
        fontFamily: {
          sans: ['Inter', 'ui-sans-serif', 'system-ui'],
        },
        spacing: {
          '128': '32rem',
        },
        borderRadius: {
          '4xl': '2rem',
        },
      },
    },
    plugins: [
      require('@tailwindcss/forms'),
      require('@tailwindcss/typography'),
      require('@tailwindcss/aspect-ratio'),
    ],
  };
  