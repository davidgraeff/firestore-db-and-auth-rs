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

const FIREBASE_TOKEN = "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJpc3MiOiJmaXJlYmFzZS1hZG1pbnNkay1hbDhrdEBzdXBlci1zcXVhcmVzLmlhbS5nc2VydmljZWFjY291bnQuY29tIiwic3ViIjoiZmlyZWJhc2UtYWRtaW5zZGstYWw4a3RAc3VwZXItc3F1YXJlcy5pYW0uZ3NlcnZpY2VhY2NvdW50LmNvbSIsImF1ZCI6Imh0dHBzOlwvXC9pZGVudGl0eXRvb2xraXQuZ29vZ2xlYXBpcy5jb21cL2dvb2dsZS5pZGVudGl0eS5pZGVudGl0eXRvb2xraXQudjEuSWRlbnRpdHlUb29sa2l0IiwidWlkIjoiNDg3IiwiaWF0IjoxNjM1ODcyOTAzLCJleHAiOjE2MzU4NzY1MDN9.CtfmrKXeJ6ZZxYgoFItcYRwZa6EDVxNxj3bw7pLV0Njh62FjW9VOQSUvBT7GpudhKjJUxXPNYLZkGY2u4lO2LAVNsm52T8KgEFmkHqUVF35BNlX9nLAncODdDVZn_SIMzPvLKG-LOdOjrTK3a2aoaBGsRJUJmDyMhxhe1Bd7x16m_bJMPQNCtXLUoGkNeZ17efB6WutE91Gi5h56SpPjfqRSMRM79S13Ds7cugjXvNzsvQ2WXouyiKJfY2QmkAm3LYVdCKqRz6nq3s0FwN2097e7ubNtfXet_moSDQPzNW8TkUn6vvDG5oqQNzGl9d_ew8cDg9eQd0cRlYdAPREcag"

const app = initializeApp(firebaseConfig);
const auth = getAuth();

signInWithCustomToken(auth, FIREBASE_TOKEN)
  .then((userCreds) => {
    console.log('id token');
    console.log(userCreds._tokenResponse.idToken);
  })
  .catch((err) => {
    console.log('failed with error', err);
  });
