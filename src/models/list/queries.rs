use wither::bson::doc;
use wither::bson::Bson;
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

pub fn create_find_populated_query(query: Document) -> Vec<Document> {
  let pipeline = vec![
    doc! { "$match": query },
    doc! {
      "$lookup": {
        "from": "resources",
        "as": "resources",
        "let": { "list": "$_id" },
        "pipeline": vec![
          doc! {
            "$match": {
              "$expr": { "$eq": [ "$list",  "$$list" ] },
              "completed_at": Bson::Null
            }
          },
          doc! { "$sort": { "position": 1 } },
          doc! { "$limit": 1 }
        ]
      }
    },
    doc! { "$sort": { "created_at": 1 } },
    doc! {
      "$lookup": {
        "from": "users",
        "localField": "fork.user",
        "foreignField": "_id",
        "as": "fork.user",
      }
    },
    doc! {
      "$unwind": {
        "path": "$fork.user",
        "preserveNullAndEmptyArrays": true
      }
    },
    doc! {
      "$lookup": {
        "from": "lists",
        "localField": "fork.list",
        "foreignField": "_id",
        "as": "fork.list",
      }
    },
    doc! {
      "$unwind": {
        "path": "$fork.list",
        "preserveNullAndEmptyArrays": true
      }
    },
    doc! {
      "$project": {
        "_id": false,
        "id": "$_id",
        "user": "$user",
        "title": "$title",
        "description": "$description",
        "tags": "$tags",
        "slug": "$slug",
        "is_public": "$is_public",
        "created_at": "$created_at",
        "updated_at": "$updated_at",
        "archived_at": "$archived_at",
        "fork": "$fork"
        // "fork": {
        //   "user": {
        //     "id": "$fork.user._id",
        //     "slug": "$fork.user.slug",
        //     "name": "$fork.user.name",
        //     "avatar": "$fork.user.avatar",
        //   },
        //   "list": {
        //     "id": "$fork.list._id",
        //     "title": "$fork.list.title",
        //     "slug": "$fork.list.slug",
        //   }
        // }
      }
    },
  ];

  pipeline
}
