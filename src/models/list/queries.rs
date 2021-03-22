use wither::bson::doc;
use wither::bson::Document;

pub fn create_discover_query(query: Document, skip: i64, limit: i64) -> Vec<Document> {
  let pipeline = vec![
    doc! { "$match": query },
    doc! {
      "$lookup": {
        "from":"resources",
        "as": "resources",
        "let": {
          "list": "$_id"
        },
        "pipeline": vec![
          doc! {
            "$match": {
              "$expr": {
                "$eq": [ "$list",  "$$list" ]
              }
            }
          },
          doc! {
            "$sort": {
              "created_at": -1
            }
          },
          doc! { "$limit": 1 }
        ]
      }
    },
    doc! {
      "$match": {
        "resources": { "$ne": [] }
      }
    },
    doc! {
      "$sort": {
        "created_at": -1
      }
    },
    doc! { "$skip":  skip },
    doc! { "$limit": limit },
    doc! {
      "$lookup": {
        "from":"users",
        "localField": "user",
        "foreignField": "_id",
        "as": "user",
      }
    },
    doc! { "$unwind": "$user" },
    doc! {
      "$project": {
        "_id": false,
        "id": "$_id",
        "title": "$title",
        "description": "$description",
        "tags": "$tags",
        "created_at": "$created_at",
        "slug": "$slug",
        "user": {
          "id": "$user._id",
          "slug": "$user.slug",
          "name": "$user.name",
          "avatar": "$user.avatar",
        }
      }
    },
  ];

  pipeline
}
