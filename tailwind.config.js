/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./taigaishida/**/*.{html,js}"],
  theme: {
    extend: {
      fontFamily: {
        koushiki: ["KoushikiSans", "thin"],
        main: ["MPlus1", "sans-serif"],
      },
    },
  },
  plugins: [],
};
