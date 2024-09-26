# CopyDocx

## A platform to share text or small files within your devices or with your friends.

### Table of Contents
- [Introduction](#introduction)
- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
- [Configuration](#configuration)
- [Building and Running with Docker](#building-and-running-with-docker)
- [Contributing](#contributing)
- [License](#license)
- [Contact](#contact)
- [Live Demo](#live-demo)

### Introduction
CopyDocx is a simple and efficient platform that allows you to share text or small files across your devices or with your friends. It provides a collaborative environment where multiple users can edit and share documents in real-time.

### Features
- **Collaborative Editing**: Multiple users can edit documents simultaneously.
- **File Sharing**: Share text or small files easily.
- **Download Documents**: Download the text content of the document.
- **No Login Required**: Use the tool without any login or registration.
- **Responsive Design**: Works seamlessly on both desktop and mobile devices.

### Installation
To run CopyDocx locally, you need to have Rust and Cargo installed. Follow these steps
(the build and run commands may require admin privilages to work):

1. Clone the repository:
    ```sh
    git clone https://github.com/arthurpeter/CopyDocx.git
    cd copydocx
    ```

2. Install dependencies:
    ```sh
    cargo build
    ```

3. Run the application:
    ```sh
    cargo run
    ```

### Usage
1. Open your browser and navigate to `http://localhost`.
2. Enter the address of the document you want to access or create a new one.
3. Start typing or editing the document collaboratively with others.
4. Use the "Download Text" button to download the text content of the document.
5. Use the "Add Document" button to add a document to the editor.

### Configuration
CopyDocx uses environment variables for configuration. Create a `.env` file in the root directory and add the necessary variables. For example:
```env
MONGODB_URI=mongodb://localhost:27017
```

### Building and Running with Docker
To build and run CopyDocx using Docker, follow these steps:

1. Build the Docker image:
    ```sh
    docker build -t copydocx .
    ```

2. Run the Docker container:
    ```sh
    docker run -p 80:80 copydocx
    ```

### Contributing
Contributions are welcome! Please fork the repository and submit a pull request for any improvements or bug fixes.

### License
This project is licensed under the MIT License. See the [`LICENSE`] file for details.

### Contact
If you have any feedback or questions, please contact us at:
- Email: [work.peter.arthur@gmail.com](mailto:work.peter.arthur@gmail.com)

### Live Demo
Check out the live demo of CopyDocx at [copydocx.onrender.com](https://copydocx.onrender.com)