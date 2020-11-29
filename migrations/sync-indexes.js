#! /usr/bin/env node

process.env["NODE_CONFIG_DIR"] = __dirname + "/../config/";

const { Mongoose } = require('mongoose')
const config = require('config')
const schemas = require('./schemas')

const mongoose = new Mongoose();

async function runScript() {
  console.log('Connecting to the database')

  const User = mongoose.model('User', schemas.user)
  const List = mongoose.model('List', schemas.list)
  const Resource = mongoose.model('Resource', schemas.resource)

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
  console.log(`Finished ensuring all indexes`)

  await mongoose.disconnect()
}


runScript()
  .then(function () {
    console.log(`Finished ensuring all indexes`)
  })
  .catch(function (err) {
    console.error(`Failed to run script. Error: ${err}`)
  })
