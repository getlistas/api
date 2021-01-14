
exports.name        = 'create-users';
exports.description = 'Creates the base users';

exports.isReversible = true;
exports.isIgnored    = false;

exports.up = async function (db, done) {
  const now = new Date()

  try {    
    await db.collection('users').insertMany([
      {
        _id: new ObjectId('000000000000000000000000'),
        email: 'nicolas.delvalle@gmail.com',
        password: '$2b$12$hSPUUa/umLEgIA4nCOs7N.GUoL.Oj3s6Ou6bf7orNLr4Zii4g4CcC', // Password1
        slug: 'ndelvalle',
        name: 'Nicolas Del Valle',
        avatar: null,
        verification_token: null,
        created_at: now,
        updated_at: now,
        verified_at: now,
      },
      {
        _id: new ObjectId('000000000000000000000001'),
        email: 'gillchristiang@gmail.com',
        password: '$2b$12$hSPUUa/umLEgIA4nCOs7N.GUoL.Oj3s6Ou6bf7orNLr4Zii4g4CcC', // Password1
        slug: 'gillchristian',
        name: 'Christian Gill',
        avatar: null,
        verification_token: null,
        created_at: now,
        updated_at: now,
        verified_at: now,
      }
    ]);

    done()
  } catch (err) {
    done(err);
  }
};

exports.down = async function (db, done) {
  try {
    await db.collection('users').deleteMany(
      {
        _id: {
          $in: [
            new ObjectId('000000000000000000000000'),
            new ObjectId('000000000000000000000001'),
          ]
        },
      }
    );

    done();
  } catch (err) {
    done(err);
  }
};
