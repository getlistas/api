
exports.name        = 'add-resource-tags';
exports.description = 'Adds a tags attribute to resources';

exports.isReversible = false;
exports.isIgnored = false;


exports.up = async function(db, done) {
  try {
    await db.collection('resources').updateMany(
      {
        tags: null
      },
      {
        $set: {
          tags: []
        }
      }
    );

    done()
  } catch (err) {
    done(err);
  }
};

exports.down = async function(db, done) {
  done();
};
