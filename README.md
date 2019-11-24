### Build & Run
`docker-compose up`

### Endpoints

* **POST** `/images`

  * Content-Type: 
    * application/json
    * multipart/form-data
  * Example: 
  ```json
    [
      {
        "filename": "img1_thumb.jpeg",
        "data": {
          "base64": "some_base64_img_string"
        }
      },
      {
        "filename": "img2_thumb.jpeg",
        "data": {
          "uri": "https://s3.amazonaws.com/media-p.slid.es/uploads/nercury/images/1236480/logo-v2.png"
        }
      }
   ]
  ```
* **GET** `/images/{filename}`
