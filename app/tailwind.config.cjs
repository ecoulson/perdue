const {fontFamily} = require('tailwindcss/defaultTheme');

/** @type {import('tailwindcss').Config} */
module.exports = {
    content: ['./templates/*.html'],
    variants: {
        extend: {
            display: ['focus-within']
        }
    },
    theme: {
        extend: {
            fontFamily: {
                sans: ['Inter var', ...fontFamily.sans],
            },
        },
    },
};
