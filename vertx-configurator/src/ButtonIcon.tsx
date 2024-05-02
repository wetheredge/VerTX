import type { LucideIcon } from 'lucide-solid';
import { buttonIconSize } from './index.css';

export function ButtonIcon(props: {
	class?: string;
	icon: LucideIcon;
	light?: boolean;
}) {
	return (
		<props.icon
			class={props.class}
			size={buttonIconSize}
			strokeWidth={props.light ? 2 : 3}
			aria-hidden="true"
		/>
	);
}
