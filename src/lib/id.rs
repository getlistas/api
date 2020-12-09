use actix_web::error::ErrorBadRequest;
use futures_util::future;
use wither::bson::oid::ObjectId;
pub struct ID(pub ObjectId);
pub struct ListID(pub ObjectId);

impl actix_web::FromRequest for ID {
    type Error = actix_web::Error;
    type Future = future::Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let id = req.match_info().query("id");
        let object_id = ObjectId::with_string(id);

        match object_id {
            Ok(value) => future::ok(ID(value)),
            Err(err) => future::err(ErrorBadRequest(err)),
        }
    }
}

impl actix_web::FromRequest for ListID {
    type Error = actix_web::Error;
    type Future = future::Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let id = req.match_info().query("list_id");
        let object_id = ObjectId::with_string(id);

        match object_id {
            Ok(value) => future::ok(ListID(value)),
            Err(err) => future::err(ErrorBadRequest(err)),
        }
    }
}
