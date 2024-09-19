document.getElementById('main').addEventListener("keyup", ({key}) => {
    if (key === "Enter") {
        goToDocument();
    }
})


function goToDocument() {
    const address = document.getElementById('doc-address').value.trim();
    if (address) {
        window.location.href = `/${address}`;
    }
}
