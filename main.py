from taigaishida import create_app

app = create_app()

if __name__ == "__main__":
    import os

    app.config["SEND_FILE_MAX_AGE_DEFAULT"] = 0
    app.run(host="0.0.0.0", port=os.environ.get("PORT", 6001), debug=True)
