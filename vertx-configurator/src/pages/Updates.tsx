import { createSignal } from 'solid-js';
import { API_BASE, ResponseKind, api } from '../api';
import * as styles from './Updates.css';

export default function Updates() {
	const [firmwareSize, setFirmwareSize] = createSignal(0);

	return (
		<>
			<h1>Updates</h1>

			<p>
				Progress:{' '}
				{Math.min(
					100,
					((api[ResponseKind.UpdateProgress]?.written ?? 0) /
						firmwareSize()) *
						100,
				).toFixed(2)}
			</p>

			{/* biome-ignore lint/a11y/noNoninteractiveElementToInteractiveRole: tabIndex makes this interactive */}
			<label class={styles.localUpdate} tabIndex={0} role="button">
				Upload local firmware
				<input
					type="file"
					onChange={async (event) => {
						const firmware = event.target.files?.item(0);
						if (firmware) {
							if (firmware.type !== 'application/octet-stream') {
								console.warn(
									'Possibly invalid firmware uploaded',
								);
							}

							setFirmwareSize(firmware.size);
							fetch(`http://${API_BASE}/update`, {
								method: 'POST',
								body: await firmware.arrayBuffer(),
							});
						}
					}}
				/>
			</label>
		</>
	);
}
