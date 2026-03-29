/** @type {import('tailwindcss').Config} */
export default {
	content: ['./src/**/*.{html,js,svelte,ts}'],
	theme: {
		extend: {
			colors: {
				bg: {
					primary: 'var(--bg-primary)',
					secondary: 'var(--bg-secondary)',
					card: 'var(--bg-card)',
					hover: 'var(--bg-hover)'
				},
				border: {
					DEFAULT: 'var(--border)',
					active: 'var(--border-active)'
				},
				accent: {
					DEFAULT: 'var(--accent)',
					secondary: 'var(--accent-secondary)',
					glow: 'var(--accent-glow)'
				},
				text: {
					primary: 'var(--text-primary)',
					secondary: 'var(--text-secondary)',
					muted: 'var(--text-muted)'
				},
				success: 'var(--success)',
				warning: 'var(--warning)',
				error: 'var(--error)',
				info: 'var(--info)'
			},
			backgroundImage: {
				'accent-gradient': 'var(--accent-gradient)'
			},
			borderRadius: {
				sm: 'var(--radius-sm)',
				md: 'var(--radius-md)',
				lg: 'var(--radius-lg)',
				xl: 'var(--radius-xl)'
			},
			transitionDuration: {
				micro: '150ms',
				standard: '250ms',
				dramatic: '400ms'
			},
			transitionTimingFunction: {
				'ease-out': 'ease-out'
			}
		}
	},
	plugins: []
};
