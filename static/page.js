document.addEventListener('DOMContentLoaded', function() {
    const path = window.location.pathname.substring(1);

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
            editor.value = data.text; // Set the retrieved text into the editor
            console.log('Loaded text:', data.text);

			// Check if file exists and add it to the file list
            if (data.file) {
                const fileList = document.getElementById('file-list');
                const fileName = data.file_name || 'loaded-file?'; // You can set a default name or retrieve it from metadata if available
                const fileBlob = new Blob([new Uint8Array(data.file)], { type: 'application/octet-stream' });

                // Create a list item
                const listItem = document.createElement('li');

                // Create a download link
                const downloadLink = document.createElement('a');
                downloadLink.textContent = fileName;
                downloadLink.href = URL.createObjectURL(fileBlob);
                downloadLink.download = fileName;

                // Create a delete button
                const deleteButton = document.createElement('button');
                deleteButton.textContent = '×'; // Unicode for 'X'
                deleteButton.className = 'delete-button';
                deleteButton.onclick = () => {
                    fetch('/save_file', {
                        method: 'POST',
                        headers: {
                            'Content-Type': 'application/json',
                        },
                        body: JSON.stringify({ file: null, file_name: '', path: path }),
                    })
                    .then(response => response.json())
                    .then(() => {
                        // Remove the list item from the file list
                        fileList.removeChild(listItem);
                    })
                    .catch(error => {
                        console.error('Error deleting file:', error);
                    });
                };

                // Append the download link and delete button to the list item
                listItem.appendChild(downloadLink);
                listItem.appendChild(deleteButton);

                // Append the list item to the file list
                fileList.appendChild(listItem);
            }

            // Proceed with WebSocket connection and other operations after fetch completes
            initializeWebSocket(path, editor);
        })
        .catch(error => {
            console.error('Error loading text:', error);
    	});
});

function initializeWebSocket(path, editor) {
	const socket = new WebSocket("ws://" + window.location.host + "/chat/" + path);
	let lastSentText = '';

	// Disable the editor for the first 3 seconds
    editor.disabled = true;
	loadingMessage.style.display = 'block';
    setTimeout(() => {
        editor.disabled = false;
		loadingMessage.style.display = 'none';
    }, 3000);

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
	});

	// Listen for messages from the WebSocket server
	socket.onmessage = (event) => {
		const receivedText = event.data;
		editor.value = receivedText; // Update the editor with the received text
	};

	let typingTimer; // Timer identifier
	const doneTypingInterval = 3000; // Time in ms (3 seconds)

	// Function to handle the debouncing
	function doneTyping() {
		// Get the current text from the editor
		const text = editor.value;

		if (text === lastSentText) {
			return;
		}
	
		const saveData = {
			text: text,
			path: path // Include the path in the save request
		};
	
		// Resend the data if it wasn't saved when another user entered.
		socket.send(text);
	
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
			lastSentText = text; // Update the last sent text
		})
		.catch(error => {
			console.error('Error saving text:', error);
		});
	}
	
	// Listen for input events on the editor
	editor.addEventListener('input', () => {
		// Clear the previous timer
		clearTimeout(typingTimer);

		// Set a new timer
		typingTimer = setTimeout(doneTyping, doneTypingInterval);
	});

	window.addEventListener('beforeunload', () => {
		// Send the text to the server before unloading
		const text = editor.value;

		if (text === lastSentText) {
			return;
		}
		
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
	const errorMessage = document.getElementById('error-message');

    // Reset the error message
    errorMessage.textContent = '';
	errorMessage.classList.remove('fade-out');
	errorMessage.style.transition = 'none';
	errorMessage.style.opacity = '1';
	errorMessage.offsetHeight;
    errorMessage.style.transition = '';

	// Check if files were selected
	if (fileInput.files.length === 0) return;

	// Check if more than one file was selected
    if (fileInput.files.length > 1) {
        errorMessage.textContent = 'Please select only one file!';
        setTimeout(() => {
            errorMessage.classList.add('fade-out');
        }, 4000); // Fade out after 4 seconds
        return;
    }

	// Get the first selected file
	const file = fileInput.files[0];

	// Check if the file size exceeds 1MB
    if (file.size > 1048576) {
        errorMessage.textContent = 'File size must be less than 1MB!';
        setTimeout(() => {
            errorMessage.classList.add('fade-out');
        }, 4000);
        return;
    }

	const path = window.location.pathname.substring(1);

	const reader = new FileReader();
	reader.onload = function(event) {
		const arrayBuffer = event.target.result;
		const fileContent = new Uint8Array(arrayBuffer);

		const saveData = {
			file: Array.from(fileContent), // Convert Uint8Array to Array
			file_name: file.name,
			path: path // Include the path in the save request
		};

		// Send the file content and path to the server as JSON
		fetch('/save_file', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
			},
			body: JSON.stringify(saveData), // Send the data directly
		})
		.then(response => response.json())
		.then(data => {
			if (data.success) {
				console.log('File uploaded successfully');
			} else {
				errorMessage.textContent = 'File upload failed!';
				setTimeout(() => {
					errorMessage.classList.add('fade-out');
				}, 4000);
			}
		})
		.catch(error => {
			console.error('Save error:', error);
		});
	};

	// Read the file as an ArrayBuffer
	reader.readAsArrayBuffer(file);

	// Clear existing documents
	fileList.innerHTML = '';

	// Create a list item
	const listItem = document.createElement('li');

	// Create a download link
	const downloadLink = document.createElement('a');
	downloadLink.textContent = file.name;
	downloadLink.href = URL.createObjectURL(file);
	downloadLink.download = file.name;

	// Create a delete button
	const deleteButton = document.createElement('button');
	deleteButton.textContent = '×'; // Unicode for 'X'
	deleteButton.className = 'delete-button';
	deleteButton.onclick = () => {
		fetch('/save_file', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
			},
			body: JSON.stringify({ file: null, file_name: '', path: path }),
		})
		.then(response => response.json())
		.then(data => {
			if (data.success) {
				console.log('File deleted successfully');
				listItem.remove(); // Remove the list item
				URL.revokeObjectURL(downloadLink.href); // Revoke the object URL
			} else {
				console.error('File deletion failed');
			}
		})
		.catch(error => {
			console.error('Delete error:', error);
		});
	};

	// Append the link and button to the list item
	listItem.appendChild(downloadLink);
	listItem.appendChild(deleteButton);

	// Append the list item to the file list
	fileList.appendChild(listItem);
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
