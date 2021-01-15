#! /usr/bin/env node

const pMap = require('p-map');
const cliProgress = require('cli-progress');
const { Mongoose } = require('mongoose')
const config = require('config')
var toSlugCase = require('to-slug-case')

const mongoose = new Mongoose();
const { Schema } = mongoose

const list = new Schema();
const List = mongoose.model('List', list)

async function runScript() {
  console.log('Connecting to the database')

  const progress = new cliProgress.SingleBar({}, cliProgress.Presets.shades_classic);

  mongoose.connect(config.database.uri, {
    dbName: config.database.name,
    useNewUrlParser: true,
    useCreateIndex: true,
    useUnifiedTopology: true
  })

  const collection = List.collection
  const cursor = await collection.find({ slug: null })
  const lists = await cursor.toArray();

  progress.start(lists.length, 0);

  await pMap(lists, async function (list) {
    const hasSlug = !!list.slug
    if (hasSlug) {
      return
    }

    const slug = toSlugCase(list.title);
    await collection.updateOne({ _id: list._id }, { $set: { slug } });

    progress.increment();
  }, { concurrency: 10 });

  progress.stop();
  await mongoose.disconnect();
}


runScript()
  .then(function () {
    console.log(`Finished ensuring all slugs`)
  })
  .catch(function (err) {
    console.error(`Failed to run script. Error: ${err}`)
  })
