document.addEventListener('DOMContentLoaded', function() {
    const path = window.location.pathname.substring(1);
    console.log("Path:", path);

    // Fetch the saved text from the server
    fetch(`/load/${path}`)
        .then(response => {
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }
            return response.json();
        })
        .then(data => {
            const editor = document.getElementById('text-editor');
            editor.value = data; // Set the retrieved text into the editor
            console.log('Loaded text:', data);

            // Proceed with WebSocket connection and other operations after fetch completes
            initializeWebSocket(path, editor);
        })
        .catch(error => {
            console.error('Error loading text:', error);
    	});
});

function initializeWebSocket(path, editor) {
	const socket = new WebSocket("ws://" + window.location.host + "/chat/" + path);

	socket.onopen = () => {
		console.log("WebSocket connection established:", path);
	};

	socket.onerror = (error) => {
		console.error("WebSocket error:", error);
	};

	socket.onclose = () => {
		console.log("WebSocket connection closed");
	};

	// Send text to the WebSocket server when the editor's content changes
	editor.addEventListener('input', () => {
		const text = editor.value;
		socket.send(text);
		console.log('sent: ', text);
	});

	// Listen for messages from the WebSocket server
	socket.onmessage = (event) => {
		const receivedText = event.data;
		editor.value = receivedText; // Update the editor with the received text
		console.log('Received the text: ', receivedText);
	};

	let typingTimer; // Timer identifier
	const doneTypingInterval = 3000; // Time in ms (3 seconds)

	// Function to handle the debouncing
	function doneTyping() {
		// Get the current text from the editor
		const text = editor.value;

		const saveData = {
			text: text,
			path: path // Include the path in the save request
		};

		// Send the text to the server
		fetch('/save_text', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
			},
			body: JSON.stringify(saveData), // Send the data directly
		})
		.then(response => response.json())
		.then(data => {
			console.log('Save successful:', data);
		})
		.catch(error => {
			console.error('Save error:', error);
		});
	}
	
	// Listen for input events on the editor
	editor.addEventListener('input', () => {
		// Clear the previous timer
		clearTimeout(typingTimer);

		// Set a new timer
		typingTimer = setTimeout(doneTyping, doneTypingInterval);
	});

	window.addEventListener('beforeunload', (event) => {
		// Send the text to the server before unloading
		const text = editor.value;
		
		const saveData = {
			text: text,
			path: path
		};
	
		// Use sendBeacon to ensure the data is sent even if the page is closing
		const blob = new Blob([JSON.stringify(saveData)], { type: 'application/json' });
		navigator.sendBeacon('/save_text', blob);
	
	});
}

function addDocuments() {
	const fileInput = document.getElementById('file-input');
	const fileList = document.getElementById('file-list');

	// Check if files were selected
	if (fileInput.files.length === 0) return;

	// Iterate over selected files
	for (const file of fileInput.files) {
		const listItem = document.createElement('li');
		listItem.textContent = file.name;

		// Create a delete button
		const deleteButton = document.createElement('button');
		deleteButton.textContent = '×'; // Unicode for 'X'
		deleteButton.className = 'delete-button';
		deleteButton.onclick = () => {
			listItem.remove(); // Remove the list item
		};

		// Append the button to the list item
		listItem.appendChild(deleteButton);

		// Append the list item to the file list
		fileList.appendChild(listItem);
	}

	// Clear file input to allow adding the same files again
	fileInput.value = '';
}

document.getElementById('fileModal').addEventListener("keyup", ({key}) => {
    if (key === "Enter") {
        downloadFile();
    }
})

// Function to display the modal
function openModal() {
	document.getElementById('fileModal').style.display = 'block';
}

// Function to close the modal
function closeModal() {
	document.getElementById('fileModal').style.display = 'none';
}

// Function to download the file
function downloadFile() {
	// Get the text and file name
	const text = document.getElementById('text-editor').value;
	const fileName = document.getElementById('fileNameInput').value || 'default.txt';

	// Create a Blob from the text
	const blob = new Blob([text], { type: 'text/plain' });
	const url = URL.createObjectURL(blob);

	// Create an anchor element and simulate a click to start the download
	const a = document.createElement('a');
	a.href = url;
	a.download = fileName;
	document.body.appendChild(a);
	a.click();

	// Cleanup
	document.body.removeChild(a);
	URL.revokeObjectURL(url);

	// Close the modal after downloading
	closeModal();
}

// Close the modal if the user clicks outside of it
window.onclick = function(event) {
	const modal = document.getElementById('fileModal');
	if (event.target === modal) {
		closeModal();
	}
}
