import { Menu, X } from 'lucide-solid';
import { createSignal, onMount } from 'solid-js';
import * as styles from './MenuButton.css';
import { mediaIsMobile } from './index.css';

const [isOpen, setIsOpen] = createSignal(false);
export { isOpen };

export function MenuButton() {
	return (
		<label
			for={styles.id}
			class={styles.button}
			tabIndex={0}
			// biome-ignore lint/a11y/noNoninteractiveElementToInteractiveRole: Doing this to avoid needing JS just to open the nav
			role="button"
			aria-label="Toggle navigation menu"
			onKeyPress={({ key }) => {
				if (key === ' ' || key === 'Enter') {
					closeMenu();
				}
			}}
		>
			<Menu
				size={styles.iconSize}
				strokeWidth="3"
				class={styles.iconMenu}
				aria-hidden="true"
			/>
			<X
				size={styles.iconSize}
				strokeWidth="3"
				class={styles.iconX}
				aria-hidden="true"
			/>
		</label>
	);
}

let menuStateRef!: HTMLInputElement;
export function MenuState() {
	onMount(() => {
		const media = window.matchMedia(mediaIsMobile);
		media.addEventListener('change', ({ matches: isMobile }) => {
			// Hide menu when leaving mobile design
			if (!isMobile) {
				closeMenu();
			}
		});
	});

	return (
		<input
			id={styles.id}
			type="checkbox"
			class={styles.state}
			onChange={({ target }) => setIsOpen(target.checked)}
			ref={menuStateRef}
		/>
	);
}

export function closeMenu() {
	if (menuStateRef.checked) {
		menuStateRef.checked = false;
		setIsOpen(false);
	}
}
