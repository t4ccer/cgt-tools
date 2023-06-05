/** @type {import('tailwindcss').Config} */
module.exports = {
  content: {
    files: ["*.html", "./src/**/*.rs"],
  },
  theme: {
    extend: {},
    colors: {
      "black": "#222222",
      "dark-gray": "#333333",
      "gray": "#666666",
      "light-gray": "#aaaaaa",
      "white": "#dddddd",
      "orange": "#fd971f",
      "yellow": "#e6db74",
      "purple": "#9d65ff",
      "cyan": "#4eb4fa",
      "pink": "#f92672",
      "green": "#a7e22e",
    }  
  },
  plugins: [],
}
