export default function Model(props: { params: { id: string } }) {
	return (
		<>
			<h1>Model {props.params.id}</h1>
		</>
	);
}
