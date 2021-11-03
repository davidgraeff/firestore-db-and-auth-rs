import { getAuth, signInWithCustomToken } from 'firebase/auth';
import { initializeApp } from 'firebase/app';

const firebaseConfig = {
  apiKey: "AIzaSyDMLe70XG7jAfE_rOZn76_ZgRaRNALzQzk",
  authDomain: "super-squares.firebaseapp.com",
  databaseURL: "https://super-squares.firebaseio.com",
  projectId: "super-squares",
  storageBucket: "super-squares.appspot.com",
  messagingSenderId: "286762543163",
  appId: "1:286762543163:web:ed27e9f074c3487855141e",
  measurementId: "G-QPNNJ4M2LQ"
};

const token = process.argv[2];

const app = initializeApp(firebaseConfig);
const auth = getAuth();

signInWithCustomToken(auth, token)
  .then((userCreds) => {
    console.log('id token');
    console.log(userCreds._tokenResponse.idToken);
  })
  .catch((err) => {
    console.log('failed with error', err);
  });
