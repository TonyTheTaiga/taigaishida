from taigaishida import create_app

app = create_app()

if __name__ == "__main__":
    app.config["SEND_FILE_MAX_AGE_DEFAULT"] = 0
    app.run(port=6000, debug=True)
