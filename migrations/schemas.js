const mongoose = require('mongoose')

const { Schema } = mongoose
const { ObjectId } = Schema.Types


const user = new Schema({
  email: { type: String },
  password: { type: String },
  slug: { type: String },
  name: { type: String },
  avatar: { type: String },
  verification_token: { type: String },
  created_at: { type: String },
  updated_at: { type: String },
  verified_at: { type: String },
});

user.index({ email: 1 }, { unique: true });
user.index({ slug: 1 }, { unique: true });


const list = new Schema({
  user: { type: ObjectId, ref: 'User' },
  title: { type: String },
  description: { type: String },
  created_at: { type: Date },
  updated_at: { type: Date },
});

list.index({ user: 1 });


const resource = new Schema({
  list: { type: ObjectId, ref: 'List' },
  user: { type: ObjectId, ref: 'User' },
  url: { type: String },
  title: { type: String },
  description: { type: String },
  created_at: { type: Date },
  updated_at: { type: Date },
  completed_at: { type: Date },
})

resource.index({ user: 1 })


module.exports = {
  user,
  list,
  resource
}
