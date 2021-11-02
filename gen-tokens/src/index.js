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

const FIREBASE_TOKEN = "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJpc3MiOiJmaXJlYmFzZS1hZG1pbnNkay1hbDhrdEBzdXBlci1zcXVhcmVzLmlhbS5nc2VydmljZWFjY291bnQuY29tIiwic3ViIjoiZmlyZWJhc2UtYWRtaW5zZGstYWw4a3RAc3VwZXItc3F1YXJlcy5pYW0uZ3NlcnZpY2VhY2NvdW50LmNvbSIsImF1ZCI6Imh0dHBzOlwvXC9pZGVudGl0eXRvb2xraXQuZ29vZ2xlYXBpcy5jb21cL2dvb2dsZS5pZGVudGl0eS5pZGVudGl0eXRvb2xraXQudjEuSWRlbnRpdHlUb29sa2l0IiwidWlkIjoiNDg3IiwiaWF0IjoxNjM1ODY1ODM5LCJleHAiOjE2MzU4Njk0Mzl9.hZAWWjHVpp2uTPh4oiRbsOmrHuH_NfRuPPHbThIe6oyFFnmcETC-ZaKDtJ9q2pmsRyfU-tohWm4sr7T7oDRc0bWhDVgfdKsdahCLTViAhVR_oEsLmZxXO_-uycli8DFar-tTliqlUXMM9FJLNkCh754IRld0psuXPnpllnBjMjBal5NE7_OXDflJtGAmGIOCxytuMFDsgO4081f0leE41vRIMLYmmzXkOF4-Yoj3pAh9HBzGB6k_Fk7UdnL8CUzuBR305lF35EVrZCTgACGloT7SR9jLJcYoPTmtaZUNrEAOxZAlcElh2LkrEUMEA1_-VNhJF3byyBJ-oVPbeckxsg";

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
