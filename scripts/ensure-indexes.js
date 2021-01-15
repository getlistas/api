#! /usr/bin/env node

const { Mongoose } = require('mongoose')
const config = require('config')

const mongoose = new Mongoose();
const { Schema } = mongoose
const { ObjectId } = Schema.Types


const user = new Schema();
const list = new Schema();
const resource = new Schema();


// User indexes
user.index({ email: 1 }, { unique: true });
user.index({ slug: 1 }, { unique: true });

// List indexes
list.index({ user: 1 });
list.index({ user: 1, slug: 1 });

// Resource indexes
resource.index({ user: 1 })
resource.index({ user: 1, list: 1 })


async function runScript() {
  console.log('Connecting to the database')

  const User = mongoose.model('User', user)
  const List = mongoose.model('List', list)
  const Resource = mongoose.model('Resource', resource)

  mongoose.connect(config.database.uri, {
    dbName: config.database.name,
    useNewUrlParser: true,
    useCreateIndex: true,
    useUnifiedTopology: true
  })

  console.log(`Starting ensuring indexes`)
  await User.syncIndexes()
  await List.syncIndexes()
  await Resource.syncIndexes()

  await mongoose.disconnect()
}


runScript()
  .then(function () {
    console.log(`Finished ensuring all indexes`)
  })
  .catch(function (err) {
    console.error(`Failed to run script. Error: ${err}`)
  })
